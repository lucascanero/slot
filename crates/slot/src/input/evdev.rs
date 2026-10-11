use std::path::{Path, PathBuf};

use slot_input::{Btn, RawEvent};

pub const EVENT_BYTES: usize = 24;

pub const EV_SYN: u16 = 0x00;
pub const EV_KEY: u16 = 0x01;
pub const EV_SW: u16 = 0x05;
pub const SW_LID: u16 = 0x00;

pub const EV_ABS: u16 = 0x03;
pub const ABS_HAT0X: u16 = 0x10;
pub const ABS_HAT0Y: u16 = 0x11;
pub const ABS_Z: u16 = 0x02;
pub const ABS_RZ: u16 = 0x05;
pub const SW_HEADPHONE_INSERT: u16 = 0x02;

#[derive(Default)]
pub struct Hat {
    x: i32,
    y: i32,
}

impl Hat {
    pub fn feed(&mut self, ev: Ev) -> Vec<RawEvent> {
        let (held, ends) = match ev.code {
            ABS_HAT0X => (&mut self.x, [Btn::Left, Btn::Right]),
            ABS_HAT0Y => (&mut self.y, [Btn::Up, Btn::Down]),
            _ => return Vec::new(),
        };
        let (was, now) = (*held, ev.value.signum());
        if now == was {
            return Vec::new();
        }
        *held = now;
        let btn = |v: i32| ends[usize::from(v > 0)];
        let mut out = Vec::new();
        if was != 0 {
            out.push(RawEvent::Up(btn(was)));
        }
        if now != 0 {
            out.push(RawEvent::Down(btn(now)));
        }
        out
    }
}

pub struct Trigger {
    btn: Btn,
    max: i64,
    held: bool,
}

impl Trigger {
    pub fn new(btn: Btn, max: i32) -> Self {
        Trigger {
            btn,
            max: i64::from(max.max(1)),
            held: false,
        }
    }

    pub fn feed(&mut self, value: i32) -> Option<RawEvent> {
        let v = i64::from(value);
        if !self.held && v * 2 > self.max {
            self.held = true;
            return Some(RawEvent::Down(self.btn));
        }
        if self.held && v * 4 < self.max {
            self.held = false;
            return Some(RawEvent::Up(self.btn));
        }
        None
    }
}

pub struct Sticks {
    hat: Hat,
    l2: Trigger,
    r2: Trigger,
}

impl Sticks {
    pub fn new(l2_max: i32, r2_max: i32) -> Self {
        Sticks {
            hat: Hat::default(),
            l2: Trigger::new(Btn::L2, l2_max),
            r2: Trigger::new(Btn::R2, r2_max),
        }
    }

    pub fn feed(&mut self, ev: Ev) -> Vec<RawEvent> {
        match ev.code {
            ABS_Z => self.l2.feed(ev.value).into_iter().collect(),
            ABS_RZ => self.r2.feed(ev.value).into_iter().collect(),
            _ => self.hat.feed(ev),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Ev {
    pub kind: u16,
    pub code: u16,
    pub value: i32,
}

pub fn decode(bytes: &[u8]) -> Option<Ev> {
    let packet: &[u8; EVENT_BYTES] = bytes.get(..EVENT_BYTES)?.try_into().ok()?;
    Some(Ev {
        kind: u16::from_le_bytes([packet[16], packet[17]]),
        code: u16::from_le_bytes([packet[18], packet[19]]),
        value: i32::from_le_bytes([packet[20], packet[21], packet[22], packet[23]]),
    })
}

pub fn to_raw(ev: Ev) -> Option<RawEvent> {
    if ev.kind != EV_KEY {
        return None;
    }
    let btn = code_to_btn(ev.code)?;
    match ev.value {
        0 => Some(RawEvent::Up(btn)),
        1 => Some(RawEvent::Down(btn)),
        _ => None,
    }
}

pub fn code_to_btn(code: u16) -> Option<Btn> {
    Some(match code {
        0x131 => Btn::A,
        0x130 => Btn::B,
        0x134 => Btn::X,
        0x133 => Btn::Y,
        0x136 => Btn::L1,
        0x137 => Btn::R1,
        0x13a => Btn::Select,
        0x13b => Btn::Start,
        0x13c => Btn::Menu,
        115 => Btn::VolUp,
        114 => Btn::VolDown,
        116 => Btn::Power,
        _ => return None,
    })
}

pub const WANTED: [u16; 12] = [
    0x130, 0x131, 0x133, 0x134, 0x136, 0x137, 0x13a, 0x13b, 0x13c, 115, 114, 116,
];

pub use slot_power::has_bit;

pub fn pick_devices(dev: &Path, sys: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dev) else {
        return Vec::new();
    };
    let mut nodes: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("event"))
        })
        .collect();
    nodes.sort();
    nodes.retain(|node| {
        let Some(name) = node.file_name() else {
            return false;
        };
        wanted_node(&sys.join(name).join("device"))
    });
    nodes
}

fn wanted_node(device: &Path) -> bool {
    let cap = |file: &str| std::fs::read_to_string(device.join("capabilities").join(file));
    let keys = cap("key").unwrap_or_default();
    let lid = cap("sw").is_ok_and(|sw| has_bit(&sw, SW_LID));
    lid || WANTED.iter().any(|bit| has_bit(&keys, *bit))
}

pub fn device_name(sys: &Path, node: &Path) -> String {
    node.file_name()
        .and_then(|n| std::fs::read_to_string(sys.join(n).join("device/name")).ok())
        .map(|n| n.trim().to_string())
        .unwrap_or_else(|| node.display().to_string())
}
