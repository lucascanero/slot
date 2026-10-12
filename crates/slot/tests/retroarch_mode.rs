mod common;

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use common::{app_booting_at, tmp_root_with_carts, Clock, CLOCK_IS_SET};
use slot::app::{App, GameMenu, LinkRow, Phase};
use slot::core::dylib_name;
use slot::link_net::Cancel;
use slot::link_radio::{LinkRole, RadioFail};
use slot::link_start::LinkFail;
use slot::retroarch::{RaRequest, RA_BINARY};
use slot_input::{Action, Btn};
use slot_store::{
    read_slot_state, write_selected_core, write_slot_state, Core, Layout, Platform, SlotState,
};
use slot_ui::{QuickRow, QuickValue, Toast};
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

fn install_retroarch(root: &Path) {
    let layout = Layout::flat(root);
    std::fs::create_dir_all(layout.retroarch()).unwrap();
    std::fs::write(layout.retroarch().join(RA_BINARY), b"#!/bin/sh\n").unwrap();
    std::fs::create_dir_all(layout.app()).unwrap();
    for core in Core::ALL {
        std::fs::write(layout.app().join(dylib_name(core)), b"").unwrap();
    }
}

fn game(root: &Path) -> (std::path::PathBuf, String, std::path::PathBuf) {
    match RaRequest::read(root) {
        Some(RaRequest::Game {
            core,
            core_name,
            rom,
            ..
        }) => (core, core_name, rom),
        other => panic!("no game request on the card: {other:?}"),
    }
}

#[test]
fn a_cart_put_in_in_retroarch_mode_goes_to_retroarch_and_slot_leaves() {
    let (d, mut a, _) = on_carousel(true);
    install_retroarch(d.path());
    a.apply(Action::Insert);
    assert!(a.exiting(), "slot did not leave for RetroArch");
    assert!(
        matches!(a.phase(), Phase::Shelf),
        "slot started the game itself: {:?}",
        a.phase()
    );
    let (core, name, rom) = game(d.path());
    assert_eq!(
        core,
        Layout::flat(d.path()).app().join(dylib_name(Core::Mgba))
    );
    assert_eq!(name, "mGBA");
    assert_eq!(rom, d.path().join("Games/GBA/Emerald.gba"));
    assert_eq!(read_slot_state(d.path()).cart.as_deref(), Some("Emerald"));
}

#[test]
fn retroarch_loads_the_core_chosen_on_the_cart() {
    let (d, mut a, _) = on_carousel(true);
    install_retroarch(d.path());
    write_selected_core(d.path(), "Emerald", Core::Gpsp).unwrap();
    a.apply(Action::Insert);
    let (core, name, _) = game(d.path());
    assert_eq!(
        core,
        Layout::flat(d.path()).app().join(dylib_name(Core::Gpsp))
    );
    assert_eq!(name, "gpSP");
}

#[test]
fn a_game_boy_cart_goes_to_mgba_whatever_was_written_for_it() {
    let d = common::tmp_root_with_gb_carts(&["Tetris"]);
    install_retroarch(d.path());
    write_selected_core(d.path(), "Tetris", Core::Gpsp).unwrap();
    let (mut a, _) = booted_with(
        d.path(),
        SlotState {
            use_retroarch: true,
            ..SlotState::default()
        },
    );
    a.apply(Action::Insert);
    assert_eq!(game(d.path()).1, "mGBA");
}

#[test]
fn with_no_retroarch_on_the_card_the_cart_is_refused_and_slot_stays() {
    let (d, mut a, _) = on_carousel(true);
    a.apply(Action::Insert);
    assert!(!a.exiting(), "slot left for a RetroArch that is not there");
    assert!(a.refusal_active(a.now()), "no refusal");
    assert_eq!(RaRequest::read(d.path()), None);
}

#[test]
fn with_the_core_missing_the_cart_is_refused() {
    let (d, mut a, _) = on_carousel(true);
    install_retroarch(d.path());
    std::fs::remove_file(Layout::flat(d.path()).app().join(dylib_name(Core::Mgba))).unwrap();
    a.apply(Action::Insert);
    assert!(!a.exiting());
    assert!(a.refusal_active(a.now()));
    assert_eq!(RaRequest::read(d.path()), None);
}

#[test]
fn retroarch_settings_opens_retroarch_with_no_game() {
    let (d, mut a, _) = on_carousel(true);
    install_retroarch(d.path());
    walk_to(&mut a, QuickRow::RetroArchSettings);
    press(&mut a, Btn::A);
    assert!(a.exiting(), "A on RetroArch Settings did not leave");
    assert_eq!(RaRequest::read(d.path()), Some(RaRequest::Menu));
}

#[test]
fn retroarch_settings_without_retroarch_is_refused() {
    let (d, mut a, _) = on_carousel(true);
    walk_to(&mut a, QuickRow::RetroArchSettings);
    press(&mut a, Btn::A);
    assert!(!a.exiting());
    assert!(a.refusal_active(a.now()));
    assert_eq!(RaRequest::read(d.path()), None);
}

#[test]
fn coming_back_from_retroarch_shows_the_shelf_on_that_cart_without_putting_it_in() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let (a, _) = booted_with(
        d.path(),
        SlotState {
            use_retroarch: true,
            cart: Some("Fusion".into()),
            cart_platform: Some(Platform::Gba),
            ..SlotState::default()
        },
    );
    assert!(
        matches!(a.phase(), Phase::Shelf),
        "slot put the cart in on its own: {:?}",
        a.phase()
    );
    assert_eq!(a.selected_stem(), Some("Fusion"));
    assert!(!a.exiting(), "slot went straight back to RetroArch");
}

#[test]
fn turning_retroarch_off_after_a_hand_off_does_not_put_the_cart_in_next_boot() {
    let (d, mut a, _) = on_carousel(true);
    install_retroarch(d.path());
    a.apply(Action::Insert);
    assert!(a.exiting());
    std::fs::remove_file(slot::retroarch::request_path(d.path())).unwrap();
    let (mut back, _) = app_booting_at(d.path(), CLOCK_IS_SET);
    walk_to(&mut back, QuickRow::RetroArch);
    press(&mut back, Btn::Left);
    let (later, _) = app_booting_at(d.path(), CLOCK_IS_SET);
    assert!(
        matches!(later.phase(), Phase::Shelf),
        "Slot mode put RetroArch's last cart in: {:?}",
        later.phase()
    );
}

#[test]
fn slot_mode_still_plays_the_cart_itself() {
    let (d, mut a, _) = on_carousel(false);
    install_retroarch(d.path());
    a.apply(Action::Insert);
    assert!(
        matches!(a.phase(), Phase::Inserting { .. }),
        "{:?}",
        a.phase()
    );
    assert!(!a.exiting());
    assert_eq!(RaRequest::read(d.path()), None);
}

fn settle(a: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while matches!(a.game_menu(), Some(GameMenu::Working { .. })) {
        a.update(1.0 / 60.0);
        std::thread::sleep(Duration::from_millis(1));
        assert!(Instant::now() < deadline, "the overlay never left Working");
    }
}

fn linked(d: &Path) -> (Option<LinkRole>, Vec<(String, String)>) {
    match RaRequest::read(d) {
        Some(RaRequest::Game { link, options, .. }) => (link, options),
        other => panic!("no game request on the card: {other:?}"),
    }
}

#[test]
fn select_on_a_cart_opens_host_and_join_only_in_retroarch_mode() {
    let (_d, mut a, _) = on_carousel(true);
    a.apply(Action::GbaDown(Btn::Select));
    assert_eq!(a.game_menu(), Some(GameMenu::Pick(LinkRow::Host)));
    let (_d, mut slot_mode, _) = on_carousel(false);
    slot_mode.apply(Action::GbaDown(Btn::Select));
    assert_eq!(slot_mode.game_menu(), None, "Slot mode's shelf changed");
}

#[test]
fn host_brings_the_radio_up_then_hands_off_with_the_link() {
    let (d, mut a, _) = on_carousel(true);
    install_retroarch(d.path());
    a.set_ra_radio(Arc::new(|role, _: &Cancel| {
        assert_eq!(role, LinkRole::Host);
        Ok(())
    }));
    a.apply(Action::GbaDown(Btn::Select));
    press(&mut a, Btn::A);
    assert!(matches!(a.game_menu(), Some(GameMenu::Working { .. })));
    settle(&mut a);
    assert!(a.exiting(), "slot never left for RetroArch");
    assert_eq!(a.game_menu(), None);
    let (link, options) = linked(d.path());
    assert_eq!(link, Some(LinkRole::Host));
    assert_eq!(
        options,
        slot::retroarch::link_options(Core::Mgba, LinkRole::Host, "auto")
    );
}

#[test]
fn join_is_player_one() {
    let (d, mut a, _) = on_carousel(true);
    install_retroarch(d.path());
    a.set_ra_radio(Arc::new(|_, _: &Cancel| Ok(())));
    a.apply(Action::GbaDown(Btn::Select));
    press(&mut a, Btn::Right);
    press(&mut a, Btn::A);
    settle(&mut a);
    let (link, options) = linked(d.path());
    assert_eq!(link, Some(LinkRole::Join));
    assert!(
        options.contains(&("mgba_link_player".to_string(), "1".to_string())),
        "{options:?}"
    );
}

#[test]
fn a_join_with_no_host_shows_the_failure_and_slot_stays() {
    let (d, mut a, _) = on_carousel(true);
    install_retroarch(d.path());
    a.set_ra_radio(Arc::new(|_, _: &Cancel| Err(RadioFail::NoHost)));
    a.apply(Action::GbaDown(Btn::Select));
    press(&mut a, Btn::Right);
    press(&mut a, Btn::A);
    settle(&mut a);
    assert!(
        matches!(
            a.game_menu(),
            Some(GameMenu::Failed {
                fail: LinkFail::NobodyCame,
                ..
            })
        ),
        "{:?}",
        a.game_menu()
    );
    assert!(!a.exiting());
    assert_eq!(RaRequest::read(d.path()), None);
    press(&mut a, Btn::A);
    assert_eq!(a.game_menu(), None, "OK did not close the failure");
}

#[test]
fn b_while_the_radio_comes_up_cancels_without_handing_off() {
    let (d, mut a, _) = on_carousel(true);
    install_retroarch(d.path());
    a.set_ra_radio(Arc::new(|_, cancel: &Cancel| {
        while !cancel.is_cancelled() {
            std::thread::sleep(Duration::from_millis(2));
        }
        Err(RadioFail::Cancelled)
    }));
    a.apply(Action::GbaDown(Btn::Select));
    press(&mut a, Btn::A);
    press(&mut a, Btn::B);
    settle(&mut a);
    assert_eq!(a.game_menu(), None);
    assert!(!a.exiting());
    assert_eq!(RaRequest::read(d.path()), None);
}

#[test]
fn with_retroarch_missing_a_link_is_refused_before_the_radio_comes_up() {
    let (d, mut a, _) = on_carousel(true);
    let asked = Arc::new(AtomicBool::new(false));
    let flag = asked.clone();
    a.set_ra_radio(Arc::new(move |_, _: &Cancel| {
        flag.store(true, Ordering::SeqCst);
        Ok(())
    }));
    a.apply(Action::GbaDown(Btn::Select));
    press(&mut a, Btn::A);
    std::thread::sleep(Duration::from_millis(20));
    a.update(1.0 / 60.0);
    assert_eq!(a.game_menu(), None);
    assert!(a.refusal_active(a.now()));
    assert!(
        !asked.load(Ordering::SeqCst),
        "the radio came up for nothing"
    );
    assert_eq!(RaRequest::read(d.path()), None);
}

#[test]
fn gpsp_on_a_cart_with_no_link_says_so() {
    let (d, mut a, _) = on_carousel(true);
    write_selected_core(d.path(), "Emerald", Core::Gpsp).unwrap();
    a.apply(Action::GbaDown(Btn::Select));
    assert_eq!(a.game_menu(), None);
    assert_eq!(a.toast(), Some(Toast::NoLink));
}

#[test]
fn mgba_on_a_wireless_adapter_cart_says_to_use_gpsp() {
    let (d, mut a, _) = on_carousel(true);
    common::write_retail_header(&d, "Emerald", "POKEMON EMER", "BPEE");
    a.apply(Action::GbaDown(Btn::Select));
    assert_eq!(a.game_menu(), None);
    assert_eq!(a.toast(), Some(Toast::NeedsGpsp));
}

#[test]
fn gpsp_on_a_link_cart_carries_the_serial_slot_would_use() {
    let (d, mut a, _) = on_carousel(true);
    install_retroarch(d.path());
    common::write_retail_header(&d, "Emerald", "POKEMON EMER", "BPEE");
    write_selected_core(d.path(), "Emerald", Core::Gpsp).unwrap();
    a.set_ra_radio(Arc::new(|_, _: &Cancel| Ok(())));
    a.apply(Action::GbaDown(Btn::Select));
    press(&mut a, Btn::A);
    settle(&mut a);
    let (_, options) = linked(d.path());
    let want = a.link_mode("Emerald").1;
    assert_eq!(options, vec![("gpsp_serial".to_string(), want.to_string())]);
}
