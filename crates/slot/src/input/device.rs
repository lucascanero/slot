use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use slot_input::{Btn, InputSource, Millis, RawEvent};

use super::evdev::{
    decode, device_name, pick_devices, to_raw, Ev, Picked, Sticks, ABS_RZ, ABS_Z, EVENT_BYTES,
    EV_ABS, EV_KEY, EV_SW, EV_SYN, SW_HEADPHONE_INSERT,
};
use super::trace;

const DEV: &str = "/dev/input";
const SYS: &str = "/sys/class/input";

pub struct DeviceInput {
    pending: Arc<Mutex<Vec<RawEvent>>>,
    held: Held,
}

impl DeviceInput {
    pub fn open(root: &Path, jack: Arc<AtomicBool>) -> Self {
        DeviceInput::open_with(Path::new(DEV), Path::new(SYS), root, trace::enabled(), jack)
    }

    pub fn open_in(dev: &Path, sys: &Path, root: &Path, trace: bool) -> Self {
        DeviceInput::open_with(dev, sys, root, trace, Arc::default())
    }

    fn open_with(dev: &Path, sys: &Path, root: &Path, trace: bool, jack: Arc<AtomicBool>) -> Self {
        let trace = trace.then(|| Trace::start(root, dev, sys)).flatten();
        let pending: Arc<Mutex<Vec<RawEvent>>> = Arc::new(Mutex::new(Vec::new()));
        for picked in pick_devices(dev, sys) {
            eprintln!(
                "slot: input {} ({}) keys {:x?} axes {} jack {}",
                picked.node.display(),
                device_name(sys, &picked.node),
                picked.keys,
                picked.axes,
                picked.jack
            );
            quicken_poll(sys, &picked.node);
            let queue = pending.clone();
            let name = picked.node.display().to_string();
            let trace = trace.clone();
            let jack = jack.clone();
            let spawned = std::thread::Builder::new()
                .name(format!("slot-input-{name}"))
                .spawn(move || read_node(&picked, &queue, trace.as_deref(), &jack));
            if let Err(e) = spawned {
                eprintln!("slot: input {name}: {e}");
            }
        }
        DeviceInput {
            pending,
            held: Held::default(),
        }
    }
}

#[derive(Default)]
pub struct Held(Vec<Btn>);

impl Held {
    pub fn pass(&mut self, events: Vec<RawEvent>) -> Vec<RawEvent> {
        events
            .into_iter()
            .filter(|ev| match *ev {
                RawEvent::Down(b) if self.0.contains(&b) => false,
                RawEvent::Down(b) => {
                    self.0.push(b);
                    true
                }
                RawEvent::Up(b) => match self.0.iter().position(|h| *h == b) {
                    Some(i) => {
                        self.0.swap_remove(i);
                        true
                    }
                    None => false,
                },
            })
            .collect()
    }
}

const EVIOCGSW_8: libc::c_ulong = 0x8008_451b;

pub fn switch_on(file: &File, code: u16) -> Option<bool> {
    let mut bits = [0u8; 8];
    let rc = unsafe { libc::ioctl(file.as_raw_fd(), EVIOCGSW_8 as _, bits.as_mut_ptr()) };
    (rc >= 0).then(|| bits[usize::from(code / 8)] >> (code % 8) & 1 == 1)
}

const EVIOCGABS_BASE: libc::c_ulong = 0x8018_4540;

pub fn abs_max(file: &File, code: u16) -> Option<i32> {
    let mut info = [0i32; 6];
    let request = EVIOCGABS_BASE + libc::c_ulong::from(code);
    let rc = unsafe { libc::ioctl(file.as_raw_fd(), request as _, info.as_mut_ptr()) };
    (rc == 0 && info[2] > 0).then_some(info[2])
}

const POLL_MS: u32 = 10;

fn quicken_poll(sys: &Path, node: &Path) {
    let Some(name) = node.file_name() else {
        return;
    };
    let attr = sys.join(name).join("device/poll");
    let Some(now) = std::fs::read_to_string(&attr)
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
    else {
        return;
    };
    if now > POLL_MS {
        match std::fs::write(&attr, POLL_MS.to_string()) {
            Ok(()) => eprintln!(
                "slot: input {}: poll {now} ms -> {POLL_MS} ms",
                node.display()
            ),
            Err(e) => eprintln!("slot: input {}: poll: {e}", node.display()),
        }
    }
}

fn read_node(
    picked: &Picked,
    queue: &Mutex<Vec<RawEvent>>,
    trace: Option<&Trace>,
    jack: &AtomicBool,
) {
    let node = &picked.node;
    let label = node
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| node.display().to_string());
    let mut file = match File::open(node) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("slot: input {}: {e}", node.display());
            return;
        }
    };
    let mut sticks = Sticks::new(
        abs_max(&file, ABS_Z).unwrap_or(1),
        abs_max(&file, ABS_RZ).unwrap_or(1),
    );
    if picked.jack {
        if let Some(on) = switch_on(&file, SW_HEADPHONE_INSERT) {
            jack.store(on, Ordering::Relaxed);
        }
    }
    let mut buf = [0u8; EVENT_BYTES * 16];
    loop {
        let read = match file.read(&mut buf) {
            Ok(0) => return,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => {
                eprintln!("slot: input {}: {e}", node.display());
                return;
            }
        };
        let mut events = Vec::new();
        for ev in buf[..read].chunks_exact(EVENT_BYTES).filter_map(decode) {
            if let Some(trace) = trace {
                trace.event(&label, ev);
            }
            if !picked.wants(ev) {
                continue;
            }
            if (ev.kind == EV_KEY && ev.value == 1) || (ev.kind == EV_ABS && ev.value != 0) {
                crate::latency::read();
            }
            match ev.kind {
                EV_ABS => events.extend(sticks.feed(ev)),
                EV_SW => jack.store(ev.value != 0, Ordering::Relaxed),
                _ => events.extend(to_raw(ev)),
            }
        }
        if events.is_empty() {
            continue;
        }
        queue
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .extend(events);
    }
}

struct Trace {
    out: Mutex<File>,
    began: Instant,
}

impl Trace {
    fn start(root: &Path, dev: &Path, sys: &Path) -> Option<Arc<Trace>> {
        let path = root.join(trace::TRACE_FILE);
        let mut file = match File::create(&path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("slot: {}: {e}", path.display());
                return None;
            }
        };
        for line in trace::survey(dev, sys) {
            let _ = writeln!(file, "{line}");
        }
        let _ = writeln!(file, "--");
        let _ = file.flush();
        Some(Arc::new(Trace {
            out: Mutex::new(file),
            began: Instant::now(),
        }))
    }

    fn event(&self, node: &str, ev: Ev) {
        if ev.kind == EV_SYN {
            return;
        }
        let line = trace::event_line(node, self.began.elapsed().as_millis(), ev);
        let mut out = self.out.lock().unwrap_or_else(|e| e.into_inner());
        let _ = writeln!(out, "{line}");
        let _ = out.flush();
    }
}

impl InputSource for DeviceInput {
    fn poll(&mut self, _now: Millis) -> Vec<RawEvent> {
        let taken = std::mem::take(&mut *self.pending.lock().unwrap_or_else(|e| e.into_inner()));
        self.held.pass(taken)
    }
}
