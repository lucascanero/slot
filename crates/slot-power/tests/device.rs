use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use slot_power::{
    backlight_value, motor_change, motor_voltage, Battery, Charge, DevicePlatform, LedState,
    Platform, BACKLIGHT, MOTOR_MAX_UV, MOTOR_MIN_UV,
};
use tempfile::TempDir;

fn sysfs_with(capacity: &str, status: Option<&str>) -> TempDir {
    let d = tempfile::tempdir().unwrap();
    let supply = d.path().join("class/power_supply");
    fs::create_dir_all(supply.join("axp2202-usb")).unwrap();
    fs::write(supply.join("axp2202-usb/type"), "USB\n").unwrap();
    fs::create_dir_all(supply.join("axp2202-battery")).unwrap();
    fs::write(supply.join("axp2202-battery/type"), "Battery\n").unwrap();
    fs::write(
        supply.join("axp2202-battery/capacity"),
        format!("{capacity}\n"),
    )
    .unwrap();
    if let Some(s) = status {
        fs::write(supply.join("axp2202-battery/status"), format!("{s}\n")).unwrap();
    }
    let gpio = d.path().join("class/gpio/gpio227");
    fs::create_dir_all(&gpio).unwrap();
    fs::write(gpio.join("value"), "0\n").unwrap();
    fs::write(gpio.join("direction"), "in\n").unwrap();
    fs::create_dir_all(d.path().join("class/motor")).unwrap();
    fs::write(d.path().join("class/motor/voltage"), "3300000\n").unwrap();
    d
}

fn sysfs(capacity: &str) -> TempDir {
    sysfs_with(capacity, Some("Discharging"))
}

fn charge_of(status: Option<&str>) -> Charge {
    platform(&sysfs_with("87", status)).charge()
}

fn read(d: &TempDir, path: &str) -> String {
    fs::read_to_string(d.path().join(path))
        .unwrap()
        .trim()
        .to_string()
}

fn platform(d: &TempDir) -> DevicePlatform {
    platform_jacked(d, Arc::default())
}

fn platform_jacked(d: &TempDir, jack: Arc<AtomicBool>) -> DevicePlatform {
    DevicePlatform::probe(
        d.path(),
        &d.path().join("dev-disp"),
        PathBuf::from("/mnt/SDCARD/Slot"),
        jack,
    )
}

#[test]
fn the_backlight_follows_the_bricks_own_curve() {
    assert_eq!(backlight_value(0), 0, "step zero is dark, for doze");
    assert_eq!(backlight_value(1), 8);
    assert_eq!(backlight_value(9), 255);
    assert_eq!(backlight_value(200), 255, "a step past the top is the top");
    assert!(
        BACKLIGHT.windows(2).all(|w| w[0] < w[1]),
        "the curve only rises"
    );
}

#[test]
fn a_backlight_with_no_display_device_is_a_silent_no_op() {
    let mut p = platform(&sysfs("87"));
    p.set_backlight(5);
}

#[test]
fn the_battery_is_found_past_the_supplies_that_are_not_one() {
    let d = sysfs("87");
    assert_eq!(platform(&d).battery().map(|b| b.percent), Some(87));
}

#[test]
fn a_gauge_that_reads_as_nonsense_is_no_reading_rather_than_zero() {
    let d = sysfs("not a number");
    assert!(platform(&d).battery().is_none());
}

#[test]
fn a_tree_with_neither_a_panel_nor_a_gauge_still_boots() {
    let d = tempfile::tempdir().unwrap();
    let mut p = platform(&d);
    p.set_backlight(9);
    assert!(p.battery().is_none());
}

#[test]
fn the_three_states_the_kernel_names_are_taken_at_face_value() {
    assert_eq!(charge_of(Some("Charging")), Charge::Charging);
    assert_eq!(charge_of(Some("Discharging")), Charge::Discharging);
    assert_eq!(charge_of(Some("Full")), Charge::Full);
}

#[test]
fn everything_else_is_unknown_rather_than_a_guess() {
    assert_eq!(charge_of(Some("Not charging")), Charge::Unknown);
    assert_eq!(charge_of(Some("Unknown")), Charge::Unknown);
    assert_eq!(charge_of(Some("")), Charge::Unknown);
    assert_eq!(charge_of(None), Charge::Unknown);
}

#[test]
fn the_gauge_and_the_charge_state_come_back_together() {
    let d = sysfs_with("87", Some("Charging"));
    assert_eq!(
        platform(&d).battery(),
        Some(Battery {
            percent: 87,
            charge: Charge::Charging,
        })
    );
}

#[test]
fn a_tree_with_no_gauge_reads_as_unknown_rather_than_absent() {
    let d = tempfile::tempdir().unwrap();
    let p = platform(&d);
    assert_eq!(p.battery(), None);
    assert_eq!(p.charge(), Charge::Unknown);
}

#[test]
fn the_probe_reports_what_it_found() {
    let r = platform(&sysfs("87")).report();
    assert!(r.contains("axp2202-battery"), "{r}");
    assert!(r.contains("motor gpio227"), "{r}");
}

#[test]
fn a_probe_that_found_nothing_says_so_rather_than_saying_nothing() {
    let d = tempfile::tempdir().unwrap();
    let r = platform(&d).report();
    assert!(r.contains("no battery"), "{r}");
    assert!(r.contains("no rtc"), "{r}");
    assert!(r.contains("no motor"), "{r}");
}

#[test]
fn the_report_says_whether_there_is_an_rtc() {
    let d = sysfs("87");
    assert!(platform(&d).report().contains("no rtc"));
    fs::create_dir_all(d.path().join("class/rtc/rtc0")).unwrap();
    assert!(platform(&d).report().contains("rtc0"));
}

#[test]
fn only_the_edge_between_still_and_moving_reaches_the_motor() {
    assert_eq!(motor_change(0, false), None);
    assert_eq!(motor_change(1, false), Some(true));
    assert_eq!(motor_change(65_535, true), None);
    assert_eq!(motor_change(1, true), None);
    assert_eq!(motor_change(0, true), Some(false));
}

#[test]
fn the_motor_runs_through_the_gpio_and_its_voltage() {
    let d = sysfs("87");
    let mut p = platform(&d);
    assert_eq!(read(&d, "class/gpio/gpio227/direction"), "out");
    p.set_rumble(u16::MAX / 2);
    assert_eq!(read(&d, "class/gpio/gpio227/value"), "1");
    let uv: u32 = read(&d, "class/motor/voltage").parse().unwrap();
    assert!(
        (1_850_000..=1_950_000).contains(&uv),
        "half strength wrote {uv}"
    );
    p.set_rumble(0);
    assert_eq!(read(&d, "class/gpio/gpio227/value"), "0");
}

#[test]
fn the_motor_voltage_spans_the_drivers_range() {
    assert_eq!(motor_voltage(u16::MAX), MOTOR_MAX_UV);
    assert_eq!(motor_voltage(0), MOTOR_MAX_UV);
    assert!(motor_voltage(1) < MOTOR_MIN_UV + 100);
}

#[test]
fn a_motor_gpio_not_yet_exported_is_exported() {
    let d = tempfile::tempdir().unwrap();
    fs::create_dir_all(d.path().join("class/gpio")).unwrap();
    fs::write(d.path().join("class/gpio/export"), "").unwrap();
    let p = platform(&d);
    assert_eq!(read(&d, "class/gpio/export"), "227");
    assert!(p.report().contains("no motor"), "the gpio never appeared");
}

#[test]
fn headphones_follow_the_jack_switch_the_input_reads() {
    let d = sysfs("87");
    let jack = Arc::new(AtomicBool::new(false));
    let p = platform_jacked(&d, jack.clone());
    assert!(!p.headphones());
    jack.store(true, Ordering::Relaxed);
    assert!(p.headphones());
}

const MOUNTS: &str = "\
/dev/mmcblk0p5 / ext4 rw,noatime 0 0
/dev/mmcblk0p7 /mnt/sdcard vfat ro,noatime,fmask=0022,errors=remount-ro 0 0
tmpfs /mnt/sdcard/System/slot tmpfs rw,relatime 0 0
/dev/mmcblk1p1 /mnt/other exfat rw,noatime,errors=remount-ro 0 0
";

#[test]
fn a_card_the_kernel_remounted_read_only_reads_as_read_only() {
    assert!(slot_power::read_only_in(MOUNTS, Path::new("/mnt/sdcard")));
}

#[test]
fn the_closest_mount_decides_not_the_root() {
    assert!(!slot_power::read_only_in(MOUNTS, Path::new("/mnt/other")));
    assert!(!slot_power::read_only_in(
        MOUNTS,
        Path::new("/mnt/sdcard/System/slot")
    ));
}

#[test]
fn an_option_that_merely_mentions_ro_is_not_read_only() {
    let m = "/dev/x /mnt/card vfat rw,errors=remount-ro 0 0\n";
    assert!(!slot_power::read_only_in(m, Path::new("/mnt/card")));
}

fn led_tree(d: &TempDir) {
    let dir = d.path().join("class/led_anim");
    fs::create_dir_all(&dir).unwrap();
    for (attr, value) in [
        ("max_scale_f1f2", "100"),
        ("effect_f1", "1"),
        ("effect_f2", "1"),
        ("effect_m", "2"),
        ("effect_lr", "3"),
        ("effect_rgb_hex_f1", "FFFFFF"),
        ("effect_rgb_hex_f2", "FFFFFF"),
        ("effect_rgb_hex_m", "00FFFF"),
        ("effect_rgb_hex_lr", "FF00FF"),
        ("effect_cycles_f1", "-1"),
        ("effect_cycles_f2", "-1"),
        ("effect_cycles_m", "-1"),
        ("effect_cycles_lr", "-1"),
    ] {
        fs::write(dir.join(attr), format!("{value}\n")).unwrap();
    }
}

#[test]
fn only_the_f1_and_f2_buttons_light_up() {
    let d = sysfs("87");
    led_tree(&d);
    let mut p = platform(&d);
    assert_eq!(read(&d, "class/led_anim/effect_rgb_hex_m"), "000000");
    assert_eq!(read(&d, "class/led_anim/effect_rgb_hex_lr"), "000000");
    assert_eq!(read(&d, "class/led_anim/effect_m"), "4");
    assert_eq!(read(&d, "class/led_anim/effect_lr"), "4");
    assert_eq!(read(&d, "class/led_anim/max_scale_f1f2"), "40");
    for (state, hex) in [
        (LedState::Running, "00FF00"),
        (LedState::Charging, "FF8C00"),
        (LedState::Low, "FF0000"),
        (LedState::Charged, "00FF00"),
        (LedState::Off, "000000"),
    ] {
        p.set_led(state);
        for zone in ["f1", "f2"] {
            assert_eq!(
                read(&d, &format!("class/led_anim/effect_rgb_hex_{zone}")),
                hex,
                "{state:?} on {zone}"
            );
            assert_eq!(read(&d, &format!("class/led_anim/effect_{zone}")), "4");
        }
        assert_eq!(
            read(&d, "class/led_anim/effect_rgb_hex_m"),
            "000000",
            "{state:?} lit the top bar"
        );
    }
}

#[test]
fn release_puts_the_leds_back_the_way_they_were() {
    let d = sysfs("87");
    led_tree(&d);
    let mut p = platform(&d);
    p.set_led(LedState::Low);
    p.release();
    for (attr, value) in [
        ("max_scale_f1f2", "100"),
        ("effect_f1", "1"),
        ("effect_m", "2"),
        ("effect_lr", "3"),
        ("effect_rgb_hex_f1", "FFFFFF"),
        ("effect_rgb_hex_m", "00FFFF"),
        ("effect_rgb_hex_lr", "FF00FF"),
    ] {
        assert_eq!(read(&d, &format!("class/led_anim/{attr}")), value, "{attr}");
    }
}

#[test]
fn release_stops_a_running_motor() {
    let d = sysfs("87");
    let mut p = platform(&d);
    p.set_rumble(u16::MAX);
    p.release();
    assert_eq!(read(&d, "class/gpio/gpio227/value"), "0");
}

#[test]
fn a_tree_with_no_led_anim_is_a_silent_no_op() {
    let mut p = platform(&sysfs("87"));
    p.set_led(LedState::Running);
    p.release();
}
