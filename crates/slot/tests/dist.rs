mod common;

use tempfile::tempdir;

#[test]
fn ensure_creates_every_folder_dirs_names_and_nothing_else() {
    let d = tempdir().unwrap();
    let out = d.path().join("dist");
    slot::root::ensure(&out);

    for name in slot::root::DIRS {
        assert!(out.join(name).is_dir(), "{name} was not created");
    }

    let mut got: Vec<String> = std::fs::read_dir(&out)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    got.sort();
    let mut want: Vec<String> = slot::root::DIRS
        .iter()
        .map(|s| s.split('/').next().unwrap().to_string())
        .collect();
    want.sort();
    want.dedup();
    assert_eq!(got, want);
}

#[test]
fn ensure_leaves_existing_content_alone() {
    let d = tempdir().unwrap();
    let out = d.path().join("dist");
    std::fs::create_dir_all(out.join("Games")).unwrap();
    std::fs::write(out.join("Games/Emerald.gba"), b"rom").unwrap();
    slot::root::ensure(&out);
    assert_eq!(
        std::fs::read(out.join("Games/Emerald.gba")).unwrap(),
        b"rom"
    );
}

#[test]
fn a_booted_app_root_has_the_same_folders() {
    let d = common::tmp_root_with_carts(&["Emerald"]);
    for name in slot::root::DIRS {
        assert!(d.path().join(name).is_dir(), "app root is missing {name}");
    }
}

#[test]
fn a_stockmix_card_gets_the_data_folders_and_no_rom_or_system_folders() {
    let d = tempdir().unwrap();
    let layout = slot_store::Layout::stockmix(d.path(), &d.path().join("Apps/Slot"));
    slot::root::ensure_layout(&layout);
    for name in slot::root::DATA_DIRS {
        assert!(
            d.path().join("Slot").join(name).is_dir(),
            "{name} was not created"
        );
    }
    assert!(!d.path().join("Roms").exists(), "ensure made a Roms folder");
    assert!(!d.path().join("Slot/Games").exists());
    assert!(!d.path().join("Slot/System").exists());
}
