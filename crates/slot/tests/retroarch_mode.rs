mod common;

use std::path::Path;

use common::{app_booting_at, tmp_root_with_carts, Clock, CLOCK_IS_SET};
use slot::app::{App, Phase};
use slot_input::{Action, Btn};
use slot_store::{read_slot_state, write_slot_state, Platform, SlotState};
use slot_ui::{QuickRow, QuickValue};
use tempfile::TempDir;

fn booted_with(d: &Path, state: SlotState) -> (App, Clock) {
    write_slot_state(
        d,
        &SlotState {
            clock_set: true,
            ..state
        },
    )
    .expect("write slot.state");
    app_booting_at(d, CLOCK_IS_SET)
}

fn on_carousel(use_retroarch: bool) -> (TempDir, App, Clock) {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let (a, clock) = booted_with(
        d.path(),
        SlotState {
            use_retroarch,
            ..SlotState::default()
        },
    );
    assert!(
        matches!(a.phase(), Phase::Shelf),
        "not on the carousel: {:?}",
        a.phase()
    );
    (d, a, clock)
}

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

fn walk_to(a: &mut App, row: QuickRow) {
    if a.quick_menu().is_none() {
        a.apply(Action::QuickMenu);
    }
    for _ in 0..=QuickRow::MAIN.len() {
        if a.quick_menu() == Some(row) {
            return;
        }
        press(a, Btn::Down);
    }
    panic!("the bar never reached {row:?}");
}

#[test]
fn use_retroarch_stands_above_exit_and_starts_off() {
    let (_d, mut a, _) = on_carousel(false);
    walk_to(&mut a, QuickRow::RetroArch);
    assert_eq!(a.quick_value(QuickRow::RetroArch), Some(QuickValue::Off));
    press(&mut a, Btn::Down);
    assert_eq!(a.quick_menu(), Some(QuickRow::Exit));
}

#[test]
fn turning_it_on_saves_and_swaps_screen_and_gameplay_for_retroarch_settings() {
    let (d, mut a, _) = on_carousel(false);
    walk_to(&mut a, QuickRow::RetroArch);
    press(&mut a, Btn::Right);
    assert!(
        read_slot_state(d.path()).use_retroarch,
        "never reached the card"
    );
    assert_eq!(a.quick_value(QuickRow::RetroArch), Some(QuickValue::On));
    assert_eq!(
        a.quick_menu(),
        Some(QuickRow::RetroArch),
        "the bar left the row it changed"
    );
    let mut seen = Vec::new();
    for _ in 0..QuickRow::RETROARCH.len() {
        seen.push(a.quick_menu().unwrap());
        press(&mut a, Btn::Down);
    }
    assert_eq!(seen, QuickRow::RETROARCH.to_vec());
}

#[test]
fn the_menu_opens_on_use_retroarch_in_retroarch_mode() {
    let (_d, mut a, _) = on_carousel(true);
    a.apply(Action::QuickMenu);
    assert_eq!(a.quick_menu(), Some(QuickRow::RetroArch));
}

#[test]
fn b_on_start_on_boot_in_retroarch_mode_closes_the_menu() {
    let (_d, mut a, _) = on_carousel(true);
    walk_to(&mut a, QuickRow::StartOnBoot);
    press(&mut a, Btn::B);
    assert!(
        matches!(a.phase(), Phase::Shelf),
        "B went to a Gameplay page RetroArch mode does not show: {:?}",
        a.phase()
    );
}

#[test]
fn turning_it_off_brings_the_slot_pages_back_and_forgets_the_remembered_cart() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let (mut a, _) = booted_with(
        d.path(),
        SlotState {
            use_retroarch: true,
            cart: Some("Fusion".into()),
            cart_platform: Some(Platform::Gba),
            ..SlotState::default()
        },
    );
    walk_to(&mut a, QuickRow::RetroArch);
    press(&mut a, Btn::Left);
    let s = read_slot_state(d.path());
    assert!(!s.use_retroarch);
    assert_eq!(
        s.cart, None,
        "Slot mode would insert the remembered cart on boot"
    );
    press(&mut a, Btn::Down);
    assert_eq!(a.quick_menu(), Some(QuickRow::Exit));
}
