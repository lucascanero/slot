use std::ffi::{c_int, c_ulong, c_void};
use std::fs;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::{Battery, Charge, LedState, Platform};

const RUN_DIR: &str = "/run";

pub fn uptime_seconds(s: &str) -> Option<String> {
    let first = s.split_whitespace().next()?;
    if !first.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }
    if !first.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(first.to_string())
}

pub fn record_first_frame(run: &Path, uptime: &str) {
    let marker = run.join("boot-first-frame");
    let again = marker.exists();
    if !again {
        let _ = fs::write(&marker, format!("{uptime}\n"));
    }
    let label = if again {
        "first-frame-again"
    } else {
        "first-frame"
    };
    if let Ok(mut f) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(run.join("boot-trace"))
    {
        let _ = writeln!(f, "{uptime} {label}");
    }
}

pub fn trace_first_frame() {
    let Ok(raw) = fs::read_to_string("/proc/uptime") else {
        return;
    };
    let Some(up) = uptime_seconds(&raw) else {
        return;
    };
    record_first_frame(Path::new(RUN_DIR), &up);
}

pub const BACKLIGHT: [u32; 10] = [0, 8, 16, 32, 48, 72, 96, 128, 176, 255];

const DISP_LCD_SET_BRIGHTNESS: c_ulong = 0x102;
const DISP_LCD_GET_BRIGHTNESS: c_ulong = 0x103;

pub const RUMBLE_GPIO: u32 = 227;
pub const MOTOR_MIN_UV: u32 = 500_000;
pub const MOTOR_MAX_UV: u32 = 3_300_000;

pub fn backlight_value(step: u8) -> u32 {
    BACKLIGHT[usize::from(step).min(BACKLIGHT.len() - 1)]
}

pub fn motor_voltage(strength: u16) -> u32 {
    match strength {
        0 | u16::MAX => MOTOR_MAX_UV,
        s => {
            let span = u64::from(MOTOR_MAX_UV - MOTOR_MIN_UV);
            MOTOR_MIN_UV + (u64::from(s) * span / u64::from(u16::MAX)) as u32
        }
    }
}

extern "C" {
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
}

fn disp(dev: &Path, request: c_ulong, value: u32) -> Option<i32> {
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(dev)
        .ok()?;
    let mut args: [c_ulong; 4] = [0, c_ulong::from(value), 0, 0];
    let rc = unsafe { ioctl(file.as_raw_fd(), request, args.as_mut_ptr() as *mut c_void) };
    (rc >= 0).then_some(rc)
}

pub const LED_STATIC: u32 = 4;
pub const LED_SCALE_F1F2: u32 = 40;
const LED_LIT: [&str; 2] = ["f1", "f2"];
const LED_DARK: [&str; 2] = ["m", "lr"];

pub fn led_hex(state: LedState) -> &'static str {
    match state {
        LedState::Off => "000000",
        LedState::Running | LedState::Charged => "00FF00",
        LedState::Low => "FF0000",
        LedState::Charging => "FF8C00",
    }
}

fn led_attrs() -> Vec<String> {
    let mut out = vec!["max_scale_f1f2".to_string()];
    for zone in ["f1", "f2", "m", "lr"] {
        for attr in ["effect_rgb_hex", "effect_cycles", "effect"] {
            out.push(format!("{attr}_{zone}"));
        }
    }
    out
}

struct Leds {
    dir: PathBuf,
    saved: Vec<(PathBuf, String)>,
}

impl Leds {
    fn take(sysfs: &Path) -> Option<Leds> {
        let dir = sysfs.join("class/led_anim");
        if !dir.is_dir() {
            return None;
        }
        let saved = led_attrs()
            .into_iter()
            .map(|a| dir.join(a))
            .filter_map(|p| {
                let v = fs::read_to_string(&p).ok()?;
                Some((p, v.trim().to_string()))
            })
            .collect();
        let leds = Leds { dir, saved };
        for zone in LED_DARK {
            leds.colour(zone, "000000");
        }
        leds.write("max_scale_f1f2", &LED_SCALE_F1F2.to_string());
        Some(leds)
    }

    fn write(&self, attr: &str, value: &str) {
        let _ = fs::write(self.dir.join(attr), format!("{value}\n"));
    }

    fn colour(&self, zone: &str, hex: &str) {
        self.write(&format!("effect_rgb_hex_{zone}"), hex);
        self.write(&format!("effect_cycles_{zone}"), "-1");
        self.write(&format!("effect_{zone}"), &LED_STATIC.to_string());
    }

    fn show(&self, state: LedState) {
        for zone in LED_LIT {
            self.colour(zone, led_hex(state));
        }
    }

    fn restore(&self) {
        for (path, value) in &self.saved {
            let _ = fs::write(path, format!("{value}\n"));
        }
    }
}

struct GpioMotor {
    value: PathBuf,
    voltage: Option<PathBuf>,
    running: bool,
}

impl GpioMotor {
    fn open(sysfs: &Path) -> Option<GpioMotor> {
        let gpio = sysfs.join("class/gpio");
        let dir = gpio.join(format!("gpio{RUMBLE_GPIO}"));
        if !dir.is_dir() {
            let _ = fs::write(gpio.join("export"), RUMBLE_GPIO.to_string());
        }
        let value = dir.join("value");
        if !value.is_file() {
            return None;
        }
        let _ = fs::write(dir.join("direction"), "out");
        let voltage = Some(sysfs.join("class/motor/voltage")).filter(|p| p.is_file());
        Some(GpioMotor {
            value,
            voltage,
            running: false,
        })
    }

    fn play(&mut self, strength: u16) {
        let on = strength > 0;
        if on {
            if let Some(v) = &self.voltage {
                let _ = fs::write(v, motor_voltage(strength).to_string());
            }
        }
        match fs::write(&self.value, if on { "1" } else { "0" }) {
            Ok(()) => self.running = on,
            Err(e) => eprintln!("slot: rumble: {e}"),
        }
    }
}

pub struct DevicePlatform {
    root: PathBuf,
    disp: PathBuf,
    battery: Option<PathBuf>,
    rtc: Option<PathBuf>,
    motor: Option<GpioMotor>,
    leds: Option<Leds>,
    jack: Arc<AtomicBool>,
    brightness_before: Option<u32>,
    volume: Option<crate::mixer::HwVolume>,
}

impl DevicePlatform {
    pub fn new(root: PathBuf, jack: Arc<AtomicBool>) -> Self {
        let mut p = DevicePlatform::probe(Path::new("/sys"), Path::new("/dev/disp"), root, jack);
        p.brightness_before = disp(&p.disp, DISP_LCD_GET_BRIGHTNESS, 0)
            .and_then(|v| u32::try_from(v).ok())
            .filter(|v| *v > 0);
        p.volume = Some(crate::mixer::HwVolume::take(
            &crate::mixer::CliAmixer,
            Path::new("/sys"),
        ));
        p
    }

    pub fn probe(sysfs: &Path, disp: &Path, root: PathBuf, jack: Arc<AtomicBool>) -> Self {
        DevicePlatform {
            root,
            disp: disp.to_path_buf(),
            battery: first_dir(&sysfs.join("class/power_supply"), is_battery),
            rtc: first_dir(&sysfs.join("class/rtc"), |_| true),
            motor: GpioMotor::open(sysfs),
            leds: Leds::take(sysfs),
            jack,
            brightness_before: None,
            volume: None,
        }
    }

    fn breadcrumb(&self, line: &str) {
        use std::io::Write;
        if let Ok(mut f) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.join("ags-shutdown-trace.log"))
        {
            let _ = writeln!(f, "{} {line}", self.now());
            let _ = f.sync_all();
        }
    }

    pub fn trace_boot(&self) {
        self.breadcrumb("boot: platform up, trace working");
    }

    pub fn report(&self) -> String {
        let leaf = |p: &Option<PathBuf>, absent: &str| match p {
            Some(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.display().to_string()),
            None => absent.to_string(),
        };
        let motor = match &self.motor {
            Some(_) => format!("motor gpio{RUMBLE_GPIO}"),
            None => "no motor".to_string(),
        };
        let leds = if self.leds.is_some() {
            "leds led_anim"
        } else {
            "no leds"
        };
        format!(
            "backlight {}, battery {}, {}, {motor}, {leds}",
            self.disp.display(),
            leaf(&self.battery, "no battery"),
            leaf(&self.rtc, "no rtc"),
        )
    }
}

fn ran(what: &str, result: std::io::Result<std::process::ExitStatus>) {
    match result {
        Ok(status) if status.success() => eprintln!("slot: {what} ok"),
        Ok(status) => eprintln!("slot: {what} {status}"),
        Err(e) => eprintln!("slot: {what}: {e}"),
    }
}

pub fn has_bit(mask: &str, bit: u16) -> bool {
    let words: Vec<&str> = mask.split_whitespace().collect();
    let from_end = usize::from(bit) / 64;
    let Some(word) = words.len().checked_sub(from_end + 1).map(|i| words[i]) else {
        return false;
    };
    u64::from_str_radix(word, 16).is_ok_and(|w| w >> (bit % 64) & 1 == 1)
}

pub fn motor_change(strength: u16, running: bool) -> Option<bool> {
    match (strength > 0, running) {
        (true, false) => Some(true),
        (false, true) => Some(false),
        _ => None,
    }
}

fn first_dir(parent: &Path, keep: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    let mut names: Vec<PathBuf> = fs::read_dir(parent)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    names.sort();
    names.into_iter().find(|p| keep(p))
}

fn is_battery(dir: &Path) -> bool {
    fs::read_to_string(dir.join("type")).is_ok_and(|t| t.trim() == "Battery")
        && dir.join("capacity").is_file()
}

fn read_number(path: &Path) -> Option<u32> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn charge_at(dir: &Path) -> Charge {
    let Ok(text) = fs::read_to_string(dir.join("status")) else {
        return Charge::Unknown;
    };
    match text.trim() {
        "Charging" => Charge::Charging,
        "Discharging" => Charge::Discharging,
        "Full" => Charge::Full,
        _ => Charge::Unknown,
    }
}

impl Platform for DevicePlatform {
    fn headphones(&self) -> bool {
        self.jack.load(Ordering::Relaxed)
    }

    fn card_read_only(&self) -> bool {
        read_only(&self.root)
    }

    fn set_backlight(&mut self, step: u8) {
        if disp(&self.disp, DISP_LCD_SET_BRIGHTNESS, backlight_value(step)).is_none() {
            eprintln!(
                "slot: backlight: {} refused step {step}",
                self.disp.display()
            );
        }
    }

    fn battery(&self) -> Option<Battery> {
        let dir = self.battery.as_ref()?;
        let percent = read_number(&dir.join("capacity"))?;
        Some(Battery {
            percent: percent.min(100) as u8,
            charge: charge_at(dir),
        })
    }

    fn charge(&self) -> Charge {
        self.battery
            .as_ref()
            .map_or(Charge::Unknown, |dir| charge_at(dir))
    }

    fn set_led(&mut self, state: LedState) {
        if let Some(leds) = &self.leds {
            leds.show(state);
        }
    }

    fn release(&mut self) {
        if let Some(motor) = &mut self.motor {
            if motor.running {
                motor.play(0);
            }
        }
        if let Some(leds) = &self.leds {
            leds.restore();
        }
        if let Some(level) = self.brightness_before {
            let _ = disp(&self.disp, DISP_LCD_SET_BRIGHTNESS, level);
        }
        if let Some(volume) = &self.volume {
            volume.restore(&crate::mixer::CliAmixer);
        }
    }

    fn restart(&mut self) -> ! {
        self.breadcrumb("restart: reached slot, about to sync");
        let _ = Command::new("sync").status();
        self.breadcrumb("restart: sync returned, about to signal init");
        let _ = Command::new("reboot").status();
        std::thread::sleep(Duration::from_secs(10));
        std::process::exit(0)
    }

    fn poweroff(&mut self) -> ! {
        self.breadcrumb("poweroff: reached slot, about to sync");
        let _ = Command::new("sync").status();
        self.breadcrumb("poweroff: sync returned, about to signal init");
        let _ = Command::new("poweroff").status();
        self.breadcrumb("poweroff: signalled init, waiting for it to take the machine down");
        std::thread::sleep(Duration::from_secs(10));
        std::process::exit(0)
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn now(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }

    fn set_clock(&mut self, secs: i64) {
        ran(
            "date",
            Command::new("date").arg(format!("-s@{secs}")).status(),
        );
        ran(
            "hwclock",
            Command::new("hwclock").args(["-w", "-u"]).status(),
        );
    }

    fn set_rumble(&mut self, strength: u16) {
        let Some(motor) = &mut self.motor else {
            return;
        };
        if motor_change(strength, motor.running).is_some() {
            motor.play(strength);
        }
    }
}

pub fn read_only(path: &Path) -> bool {
    fs::read_to_string("/proc/mounts").is_ok_and(|m| read_only_in(&m, path))
}

pub fn read_only_in(mounts: &str, path: &Path) -> bool {
    mounts
        .lines()
        .filter_map(|l| {
            let mut f = l.split_whitespace();
            let point = Path::new(f.nth(1)?);
            let options = f.nth(1)?;
            path.starts_with(point).then_some((point, options))
        })
        .max_by_key(|(point, _)| point.as_os_str().len())
        .is_some_and(|(_, options)| options.split(',').any(|o| o == "ro"))
}
