mod common;

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use common::tmp_root_with_carts;
use slot::core::dylib_name;
use slot::link_net::Cancel;
use slot::link_radio::{LinkRole, RadioFail};
use slot::retroarch::{
    game_request, link_options, menu_request, request_path, Missing, RaLinkUp, RaRequest, RA_BINARY,
};
use slot_store::{scan, Cart, Core, Layout};

fn install_retroarch(root: &Path) {
    let layout = Layout::flat(root);
    std::fs::create_dir_all(layout.retroarch()).unwrap();
    std::fs::write(layout.retroarch().join(RA_BINARY), b"#!/bin/sh\n").unwrap();
    std::fs::create_dir_all(layout.app()).unwrap();
    for core in Core::ALL {
        std::fs::write(layout.app().join(dylib_name(core)), b"").unwrap();
    }
}

fn first_cart(root: &Path) -> Cart {
    scan(root).unwrap().into_iter().next().expect("a cart")
}

#[test]
fn a_game_request_names_the_slot_core_beside_the_app_and_the_rom() {
    let d = tmp_root_with_carts(&["Emerald"]);
    install_retroarch(d.path());
    let layout = Layout::flat(d.path());
    let cart = first_cart(d.path());
    let r = game_request(&layout, &cart, Core::Gpsp, None).unwrap();
    assert_eq!(
        r,
        RaRequest::Game {
            core: layout.app().join(dylib_name(Core::Gpsp)),
            core_name: "gpSP".into(),
            rom: cart.rom.clone(),
            link: None,
            options: Vec::new(),
        }
    );
}

#[test]
fn the_request_is_the_text_launch_sh_reads() {
    let r = RaRequest::Game {
        core: "/a/mgba_libretro.so".into(),
        core_name: "mGBA".into(),
        rom: "/r/Emerald.gba".into(),
        link: Some(LinkRole::Join),
        options: vec![
            ("mgba_link".into(), "on".into()),
            ("mgba_link_player".into(), "1".into()),
        ],
    };
    assert_eq!(
        r.to_text(),
        "mode=game\ncore=/a/mgba_libretro.so\ncore_name=mGBA\nrom=/r/Emerald.gba\nlink=join\noption=mgba_link=on\noption=mgba_link_player=1\n"
    );
    assert_eq!(RaRequest::Menu.to_text(), "mode=menu\n");
    let plain = RaRequest::Game {
        core: "/a/gpsp_libretro.so".into(),
        core_name: "gpSP".into(),
        rom: "/r/Fusion.gba".into(),
        link: None,
        options: Vec::new(),
    };
    assert!(
        plain.to_text().lines().any(|l| l == "link="),
        "no empty link line: {:?}",
        plain.to_text()
    );
}

#[test]
fn a_request_round_trips_through_the_card_with_an_awkward_rom_name() {
    let d = tmp_root_with_carts(&[]);
    let r = RaRequest::Game {
        core: "/mnt/SDCARD/Apps/Slot/mgba_libretro.so".into(),
        core_name: "mGBA".into(),
        rom: "/mnt/SDCARD/Roms/GBA/Pokémon - Emerald (U) [!] & Co's = 2.gba".into(),
        link: Some(LinkRole::Host),
        options: vec![("gpsp_serial".into(), "mul_poke".into())],
    };
    r.write(d.path()).unwrap();
    assert!(request_path(d.path()).is_file());
    assert_eq!(RaRequest::read(d.path()), Some(r));
    RaRequest::Menu.write(d.path()).unwrap();
    assert_eq!(RaRequest::read(d.path()), Some(RaRequest::Menu));
}

#[test]
fn what_is_not_a_request_reads_as_none() {
    let d = tmp_root_with_carts(&[]);
    assert_eq!(RaRequest::read(d.path()), None, "no file at all");
    for text in [
        "",
        "garbage\n",
        "mode=bogus\n",
        "mode=game\nrom=/r/x.gba\n",
        "mode=game\ncore=/a/x.so\n",
        "mode=game\ncore=\nrom=\n",
    ] {
        assert_eq!(RaRequest::parse(text), None, "{text:?} read as a request");
    }
}

#[test]
fn mgba_links_by_player_and_gpsp_by_serial() {
    assert_eq!(
        link_options(Core::Mgba, LinkRole::Host, "auto"),
        vec![
            ("mgba_link".to_string(), "on".to_string()),
            ("mgba_link_player".to_string(), "0".to_string()),
        ]
    );
    assert_eq!(
        link_options(Core::Mgba, LinkRole::Join, "auto")[1],
        ("mgba_link_player".to_string(), "1".to_string())
    );
    assert_eq!(
        link_options(Core::Gpsp, LinkRole::Join, "rfu"),
        vec![("gpsp_serial".to_string(), "rfu".to_string())]
    );
}

#[test]
fn a_linked_game_request_carries_the_role_and_its_options() {
    let d = tmp_root_with_carts(&["Emerald"]);
    install_retroarch(d.path());
    let cart = first_cart(d.path());
    let r = game_request(
        &Layout::flat(d.path()),
        &cart,
        Core::Mgba,
        Some((LinkRole::Host, "auto")),
    )
    .unwrap();
    let RaRequest::Game { link, options, .. } = r else {
        panic!("not a game request");
    };
    assert_eq!(link, Some(LinkRole::Host));
    assert_eq!(options, link_options(Core::Mgba, LinkRole::Host, "auto"));
}

#[test]
fn with_no_retroarch_or_no_core_there_is_no_request() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let layout = Layout::flat(d.path());
    let cart = first_cart(d.path());
    let ra = layout.retroarch().join(RA_BINARY);
    assert_eq!(
        game_request(&layout, &cart, Core::Mgba, None),
        Err(Missing::RetroArch(ra.clone()))
    );
    assert_eq!(menu_request(&layout), Err(Missing::RetroArch(ra)));
    install_retroarch(d.path());
    let core = layout.app().join(dylib_name(Core::Gpsp));
    std::fs::remove_file(&core).unwrap();
    assert_eq!(
        game_request(&layout, &cart, Core::Gpsp, None),
        Err(Missing::Core(core))
    );
    assert_eq!(menu_request(&layout), Ok(RaRequest::Menu));
}

fn wait(up: &mut RaLinkUp) -> Result<(), RadioFail> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(result) = up.poll() {
            return result;
        }
        assert!(Instant::now() < deadline, "the radio never answered");
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn the_radio_answer_comes_back_through_poll() {
    let mut up = RaLinkUp::spawn(
        Arc::new(|role, _: &Cancel| {
            assert_eq!(role, LinkRole::Join);
            Err(RadioFail::NoHost)
        }),
        LinkRole::Join,
    );
    assert_eq!(wait(&mut up), Err(RadioFail::NoHost));
}

#[test]
fn cancel_reaches_a_radio_that_is_still_coming_up() {
    let mut up = RaLinkUp::spawn(
        Arc::new(|_, cancel: &Cancel| {
            while !cancel.is_cancelled() {
                std::thread::sleep(Duration::from_millis(2));
            }
            Err(RadioFail::Cancelled)
        }),
        LinkRole::Host,
    );
    up.cancel();
    assert_eq!(wait(&mut up), Err(RadioFail::Cancelled));
}
