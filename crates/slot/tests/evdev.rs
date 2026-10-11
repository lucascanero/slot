use std::fs;
use std::path::Path;

use slot::input::evdev::{decode, has_bit, pick_devices, to_raw, EVENT_BYTES};
use slot_input::{Btn, RawEvent};

fn packet(kind: u16, code: u16, value: i32) -> [u8; EVENT_BYTES] {
    let mut b = [0x11u8; EVENT_BYTES];
    b[16] = kind as u8;
    b[17] = (kind >> 8) as u8;
    b[18] = code as u8;
    b[19] = (code >> 8) as u8;
    b[20..24].copy_from_slice(&value.to_le_bytes());
    b
}

const EV_KEY: u16 = 0x01;
const EV_SW: u16 = 0x05;
const EV_SYN: u16 = 0x00;
const CODE_R1: u16 = 0x137;
const KEY_VOLUMEUP: u16 = 115;
const SW_LID: u16 = 0x00;

#[test]
fn a_key_press_decodes_from_the_evdev_byte_layout() {
    let down = decode(&packet(EV_KEY, CODE_R1, 1)).and_then(to_raw);
    assert_eq!(down, Some(RawEvent::Down(Btn::R1)));
    let up = decode(&packet(EV_KEY, CODE_R1, 0)).and_then(to_raw);
    assert_eq!(up, Some(RawEvent::Up(Btn::R1)));
    let vol = decode(&packet(EV_KEY, KEY_VOLUMEUP, 1)).and_then(to_raw);
    assert_eq!(vol, Some(RawEvent::Down(Btn::VolUp)));
}

#[test]
fn autorepeat_is_not_a_press() {
    assert_eq!(decode(&packet(EV_KEY, CODE_R1, 2)).and_then(to_raw), None);
}

#[test]
fn a_lid_switch_is_not_a_button_on_the_brick() {
    assert_eq!(decode(&packet(EV_SW, SW_LID, 1)).and_then(to_raw), None);
    assert_eq!(decode(&packet(EV_SW, SW_LID, 0)).and_then(to_raw), None);
}

#[test]
fn frame_markers_and_unmapped_codes_are_dropped() {
    assert_eq!(decode(&packet(EV_SYN, 0, 0)).and_then(to_raw), None);
    assert_eq!(decode(&packet(EV_KEY, 0x2ff, 1)).and_then(to_raw), None);
}

#[test]
fn a_short_read_decodes_to_nothing_rather_than_panicking() {
    assert!(decode(&[0u8; EVENT_BYTES - 1]).is_none());
    assert!(decode(&[]).is_none());
}

#[test]
fn a_capability_bitmask_is_indexed_from_the_last_word() {
    assert!(has_bit("10 0", 68));
    assert!(!has_bit("10 0", 4));
    assert!(has_bit("8", 3));
    assert!(!has_bit("", 0));
    assert!(!has_bit("0 0 0", 300));
}

fn write(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, body).unwrap();
}

fn input_tree() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    let (dev, sys) = (d.path().join("dev/input"), d.path().join("sys/class/input"));
    for node in ["event0", "event1", "event2", "mice"] {
        write(&dev.join(node), "");
    }
    write(&sys.join("event0/device/name"), "axp2202-pek\n");
    write(&sys.join("event0/device/capabilities/key"), "0\n");
    write(&sys.join("event1/device/name"), "gpio-keys\n");
    write(
        &sys.join("event1/device/capabilities/key"),
        "1000000000000 0 0 0 0\n",
    );
    write(&sys.join("event2/device/name"), "gpio-keys-lid\n");
    write(&sys.join("event2/device/capabilities/key"), "0\n");
    write(&sys.join("event2/device/capabilities/sw"), "1\n");
    d
}

#[test]
fn devices_are_picked_by_capability_and_never_by_position() {
    let d = input_tree();
    let picked = pick_devices(
        &d.path().join("dev/input"),
        &d.path().join("sys/class/input"),
    );
    let names: Vec<_> = picked
        .iter()
        .filter_map(|p| p.node.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["event1"]);
}

#[test]
fn an_input_node_with_no_sysfs_entry_is_skipped_rather_than_opened() {
    let d = tempfile::tempdir().unwrap();
    write(&d.path().join("dev/input/event0"), "");
    let picked = pick_devices(
        &d.path().join("dev/input"),
        &d.path().join("sys/class/input"),
    );
    assert!(picked.is_empty(), "{picked:?}");
}

#[test]
fn the_survey_reports_the_nodes_that_were_not_opened_too() {
    let d = input_tree();
    let lines = slot::input::trace::survey(
        &d.path().join("dev/input"),
        &d.path().join("sys/class/input"),
    );
    let line = |node: &str| {
        lines
            .iter()
            .find(|l| l.contains(node))
            .unwrap_or_else(|| panic!("{node} was left out of the survey: {lines:?}"))
            .clone()
    };
    assert!(line("event0").contains("skipped"), "{}", line("event0"));
    assert!(
        line("event0").contains("axp2202-pek"),
        "the survey did not name the node: {}",
        line("event0")
    );
    assert!(line("event1").contains("opened"), "{}", line("event1"));
    assert!(line("event2").contains("skipped"), "{}", line("event2"));
}

#[test]
fn an_unmapped_code_is_traced_by_number_rather_than_dropped() {
    let ev = |code| slot::input::evdev::Ev {
        kind: EV_KEY,
        code,
        value: 1,
    };
    let mapped = slot::input::trace::event_line("event1", 12, ev(CODE_R1));
    assert!(mapped.contains("R1"), "{mapped}");
    let unmapped = slot::input::trace::event_line("event1", 13, ev(0x2ff));
    assert!(unmapped.contains("0x2ff"), "{unmapped}");
    assert!(
        unmapped.contains("unmapped"),
        "an unknown code has to say so, or it looks like nothing arrived: {unmapped}"
    );
}

#[test]
fn a_trace_writes_the_survey_to_the_card() {
    let d = input_tree();
    let card = tempfile::tempdir().unwrap();
    let input = slot::input::DeviceInput::open_in(
        &d.path().join("dev/input"),
        &d.path().join("sys/class/input"),
        card.path(),
        true,
    );
    drop(input);
    let log = std::fs::read_to_string(card.path().join("input-trace.log"))
        .expect("no trace file reached the card");
    assert!(log.contains("event0 skipped"), "{log}");
    assert!(log.contains("event1 opened"), "{log}");
    assert!(log.contains("gpio-keys-lid"), "{log}");
}

#[test]
fn no_trace_file_is_written_unless_one_was_asked_for() {
    let d = input_tree();
    let card = tempfile::tempdir().unwrap();
    let input = slot::input::DeviceInput::open_in(
        &d.path().join("dev/input"),
        &d.path().join("sys/class/input"),
        card.path(),
        false,
    );
    drop(input);
    assert!(!card.path().join("input-trace.log").exists());
}

#[test]
fn the_pad_is_polled_as_fast_as_the_kernel_tick_allows() {
    let d = input_tree();
    let sys = d.path().join("sys/class/input");
    write(&sys.join("event1/device/poll"), "20\n");
    write(&sys.join("event0/device/poll"), "20\n");
    let card = tempfile::tempdir().unwrap();
    let input =
        slot::input::DeviceInput::open_in(&d.path().join("dev/input"), &sys, card.path(), false);
    drop(input);
    let poll = |node: &str| std::fs::read_to_string(sys.join(node).join("device/poll")).unwrap();
    assert_eq!(
        poll("event1").trim(),
        "10",
        "the pad is still polled at 20 ms"
    );
    assert_eq!(
        poll("event0").trim(),
        "20",
        "a node slot does not read was changed"
    );
}

#[test]
fn a_pad_already_polled_faster_is_left_alone() {
    let d = input_tree();
    let sys = d.path().join("sys/class/input");
    write(&sys.join("event1/device/poll"), "5\n");
    let card = tempfile::tempdir().unwrap();
    drop(slot::input::DeviceInput::open_in(
        &d.path().join("dev/input"),
        &sys,
        card.path(),
        false,
    ));
    let poll = std::fs::read_to_string(sys.join("event1/device/poll")).unwrap();
    assert_eq!(poll.trim(), "5");
}

#[test]
fn the_pad_maps_by_the_codes_the_device_sends() {
    let expected = [
        (0x131, Btn::A),
        (0x130, Btn::B),
        (0x134, Btn::X),
        (0x133, Btn::Y),
        (0x136, Btn::L1),
        (0x137, Btn::R1),
        (0x13a, Btn::Select),
        (0x13b, Btn::Start),
        (0x13c, Btn::Menu),
        (115, Btn::VolUp),
        (114, Btn::VolDown),
        (116, Btn::Power),
    ];
    for (code, want) in expected {
        assert_eq!(
            slot::input::evdev::code_to_btn(code),
            Some(want),
            "code {code:#x}"
        );
    }
}

#[test]
fn the_menu_buttons_second_code_is_swallowed() {
    assert_eq!(slot::input::evdev::code_to_btn(0x162), None);
}

#[test]
fn the_d_pad_arrives_as_a_hat_rather_than_as_keys() {
    let mut hat = slot::input::evdev::Hat::default();
    let ev = |code, value| slot::input::evdev::Ev {
        kind: slot::input::evdev::EV_ABS,
        code,
        value,
    };
    assert_eq!(hat.feed(ev(0x11, -1)), vec![RawEvent::Down(Btn::Up)]);
    assert_eq!(hat.feed(ev(0x11, 0)), vec![RawEvent::Up(Btn::Up)]);
    assert_eq!(hat.feed(ev(0x11, 1)), vec![RawEvent::Down(Btn::Down)]);
    assert_eq!(hat.feed(ev(0x11, 0)), vec![RawEvent::Up(Btn::Down)]);
    assert_eq!(hat.feed(ev(0x10, -1)), vec![RawEvent::Down(Btn::Left)]);
    assert_eq!(hat.feed(ev(0x10, 0)), vec![RawEvent::Up(Btn::Left)]);
    assert_eq!(hat.feed(ev(0x10, 1)), vec![RawEvent::Down(Btn::Right)]);
    assert_eq!(hat.feed(ev(0x10, 0)), vec![RawEvent::Up(Btn::Right)]);
}

#[test]
fn a_hat_swung_end_to_end_releases_before_it_presses() {
    let mut hat = slot::input::evdev::Hat::default();
    let ev = |code, value| slot::input::evdev::Ev {
        kind: slot::input::evdev::EV_ABS,
        code,
        value,
    };
    assert_eq!(hat.feed(ev(0x10, -1)), vec![RawEvent::Down(Btn::Left)]);
    assert_eq!(
        hat.feed(ev(0x10, 1)),
        vec![RawEvent::Up(Btn::Left), RawEvent::Down(Btn::Right)]
    );
    assert!(hat.feed(ev(0x10, 1)).is_empty());
}

#[test]
fn the_node_carrying_only_the_power_key_is_opened() {
    let d = tempfile::tempdir().unwrap();
    let (dev, sys) = (d.path().join("dev/input"), d.path().join("sys/class/input"));
    write(&dev.join("event0"), "");
    write(&sys.join("event0/device/name"), "axp2202-pek\n");
    write(
        &sys.join("event0/device/capabilities/key"),
        "10000000000000 0\n",
    );
    assert_eq!(
        pick_devices(&dev, &sys).len(),
        1,
        "the power node was skipped"
    );
}

#[test]
fn a_hat_axis_is_traced_as_the_axis_it_is() {
    let line = slot::input::trace::event_line(
        "event1",
        9,
        slot::input::evdev::Ev {
            kind: slot::input::evdev::EV_ABS,
            code: 0x11,
            value: -1,
        },
    );
    assert!(line.contains("hat0y"), "{line}");
    assert!(!line.contains("ignored"), "{line}");
}

#[test]
fn f1_and_f2_do_nothing() {
    assert_eq!(slot::input::evdev::code_to_btn(0x13d), None);
    assert_eq!(slot::input::evdev::code_to_btn(0x13e), None);
}

#[test]
fn a_trigger_presses_past_half_and_lets_go_under_a_quarter() {
    let mut t = slot::input::evdev::Trigger::new(Btn::L2, 255);
    assert_eq!(t.feed(100), None);
    assert_eq!(t.feed(130), Some(RawEvent::Down(Btn::L2)));
    assert_eq!(t.feed(200), None);
    assert_eq!(t.feed(70), None, "between a quarter and a half it holds");
    assert_eq!(t.feed(60), Some(RawEvent::Up(Btn::L2)));
    assert_eq!(t.feed(0), None);
}

#[test]
fn a_trigger_with_no_known_range_is_pressed_by_anything_above_zero() {
    let mut t = slot::input::evdev::Trigger::new(Btn::R2, 0);
    assert_eq!(t.feed(1), Some(RawEvent::Down(Btn::R2)));
    assert_eq!(t.feed(255), None);
    assert_eq!(t.feed(0), Some(RawEvent::Up(Btn::R2)));
}

#[test]
fn the_brick_triggers_arrive_on_z_and_rz_beside_the_hat() {
    use slot::input::evdev::{Ev, Sticks, ABS_RZ, ABS_Z, EV_ABS};
    let mut s = Sticks::new(255, 255);
    let ev = |code, value| Ev {
        kind: EV_ABS,
        code,
        value,
    };
    assert_eq!(s.feed(ev(ABS_Z, 255)), vec![RawEvent::Down(Btn::L2)]);
    assert_eq!(s.feed(ev(ABS_RZ, 255)), vec![RawEvent::Down(Btn::R2)]);
    assert_eq!(s.feed(ev(0x11, -1)), vec![RawEvent::Down(Btn::Up)]);
    assert_eq!(s.feed(ev(ABS_Z, 0)), vec![RawEvent::Up(Btn::L2)]);
    assert!(
        s.feed(ev(0x00, 500)).is_empty(),
        "the left stick is not a button"
    );
}

#[test]
fn the_triggers_and_the_jack_are_traced_by_name() {
    use slot::input::evdev::{Ev, ABS_Z, EV_ABS};
    let l2 = slot::input::trace::event_line(
        "event3",
        5,
        Ev {
            kind: EV_ABS,
            code: ABS_Z,
            value: 255,
        },
    );
    assert!(l2.contains("l2"), "{l2}");
    let jack = slot::input::trace::event_line(
        "event2",
        6,
        Ev {
            kind: EV_SW,
            code: 0x02,
            value: 1,
        },
    );
    assert!(jack.contains("headphones"), "{jack}");
}

fn caps(bits: &[u16]) -> String {
    let top = bits.iter().map(|b| usize::from(*b) / 64).max().unwrap_or(0);
    let mut words = vec![0u64; top + 1];
    for b in bits {
        words[usize::from(*b) / 64] |= 1 << (b % 64);
    }
    let text: Vec<String> = words.iter().rev().map(|w| format!("{w:x}")).collect();
    text.join(" ")
        + "
"
}

fn node(d: &tempfile::TempDir, ev: &str, name: &str, keys: &[u16], abs: &[u16], sw: &[u16]) {
    let sys = d.path().join("sys/class/input").join(ev).join("device");
    write(&d.path().join("dev/input").join(ev), "");
    write(
        &sys.join("name"),
        &format!(
            "{name}
"
        ),
    );
    write(&sys.join("capabilities/key"), &caps(keys));
    write(&sys.join("capabilities/abs"), &caps(abs));
    write(&sys.join("capabilities/sw"), &caps(sw));
}

fn brick_tree() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    node(&d, "event0", "sunxi-keyboard", &[114, 115], &[], &[]);
    node(&d, "event1", "axp2202-pek", &[116], &[], &[]);
    node(
        &d,
        "event2",
        "audiocodec sunxi Audio Jack",
        &[114, 115, 226, 582],
        &[],
        &[0x02, 0x04],
    );
    node(
        &d,
        "event3",
        "TRIMUI Player1",
        &[
            59, 60, 114, 115, 0x130, 0x131, 0x133, 0x134, 0x136, 0x137, 0x13a, 0x13b, 0x13c, 0x13d,
            0x13e,
        ],
        &[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x10, 0x11],
        &[0x01],
    );
    d
}

fn picked_in(d: &tempfile::TempDir) -> Vec<slot::input::evdev::Picked> {
    pick_devices(
        &d.path().join("dev/input"),
        &d.path().join("sys/class/input"),
    )
}

fn named(picked: &[slot::input::evdev::Picked], ev: &str) -> slot::input::evdev::Picked {
    picked
        .iter()
        .find(|p| p.node.ends_with(ev))
        .unwrap_or_else(|| panic!("{ev} was not picked: {picked:?}"))
        .clone()
}

#[test]
fn the_virtual_pad_is_read_first_and_owns_the_axes() {
    let d = brick_tree();
    let picked = picked_in(&d);
    assert!(picked[0].node.ends_with("event3"), "{picked:?}");
    assert!(named(&picked, "event3").axes);
    for ev in ["event0", "event1", "event2"] {
        assert!(!named(&picked, ev).axes, "{ev} was given the axes too");
    }
}

#[test]
fn every_node_that_carries_a_button_is_read_for_it() {
    let d = brick_tree();
    let picked = picked_in(&d);
    assert_eq!(named(&picked, "event0").keys, vec![115, 114]);
    assert_eq!(named(&picked, "event1").keys, vec![116]);
    assert_eq!(named(&picked, "event2").keys, vec![115, 114]);
    let pad = named(&picked, "event3");
    assert!(pad.keys.contains(&0x131) && pad.keys.contains(&0x13c) && pad.keys.contains(&115));
}

#[test]
fn the_headphone_switch_comes_from_the_one_node_that_has_it() {
    let d = brick_tree();
    let picked = picked_in(&d);
    assert!(named(&picked, "event2").jack);
    assert!(
        !named(&picked, "event3").jack,
        "the pad's own switch is not the jack"
    );
}

#[test]
fn a_picked_node_passes_only_what_it_claimed() {
    use slot::input::evdev::{Ev, Picked, EV_ABS};
    let p = Picked {
        node: "event0".into(),
        keys: vec![115],
        axes: false,
        jack: false,
    };
    let ev = |kind, code| Ev {
        kind,
        code,
        value: 1,
    };
    assert!(p.wants(ev(EV_KEY, 115)));
    assert!(!p.wants(ev(EV_KEY, 0x130)));
    assert!(!p.wants(ev(EV_ABS, 0x10)));
    assert!(!p.wants(ev(EV_SW, 0x02)));
}

#[test]
fn a_lid_switch_alone_no_longer_opens_a_node() {
    let d = tempfile::tempdir().unwrap();
    node(&d, "event2", "gpio-keys-lid", &[], &[], &[0x00]);
    assert!(picked_in(&d).is_empty());
}

#[test]
fn a_press_seen_on_two_nodes_reaches_the_app_once() {
    let mut held = slot::input::Held::default();
    let mut out = Vec::new();
    out.extend(held.pass(vec![RawEvent::Down(Btn::VolUp)]));
    out.extend(held.pass(vec![RawEvent::Down(Btn::VolUp)]));
    out.extend(held.pass(vec![RawEvent::Up(Btn::VolUp)]));
    out.extend(held.pass(vec![RawEvent::Up(Btn::VolUp)]));
    assert_eq!(
        out,
        vec![RawEvent::Down(Btn::VolUp), RawEvent::Up(Btn::VolUp)]
    );
}

#[test]
fn a_release_with_no_press_is_dropped_and_others_pass() {
    let mut held = slot::input::Held::default();
    assert_eq!(
        held.pass(vec![
            RawEvent::Up(Btn::A),
            RawEvent::Down(Btn::B),
            RawEvent::Down(Btn::A),
            RawEvent::Up(Btn::B),
        ]),
        vec![
            RawEvent::Down(Btn::B),
            RawEvent::Down(Btn::A),
            RawEvent::Up(Btn::B)
        ]
    );
}
