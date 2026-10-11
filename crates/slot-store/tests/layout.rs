use std::fs;
use std::path::{Path, PathBuf};

use slot_store::{scan_in, Layout, Platform};

#[test]
fn stockmix_keeps_the_users_data_in_its_own_folder() {
    let l = Layout::stockmix(Path::new("/mnt/SDCARD"), Path::new("/mnt/SDCARD/Apps/Slot"));
    assert_eq!(l.data(), Path::new("/mnt/SDCARD/Slot"));
    assert_eq!(l.roms(Platform::Gba), Path::new("/mnt/SDCARD/Roms/GBA"));
    assert_eq!(l.roms(Platform::Gb), Path::new("/mnt/SDCARD/Roms/GB"));
    assert_eq!(l.roms(Platform::Gbc), Path::new("/mnt/SDCARD/Roms/GBC"));
    assert_eq!(l.app(), Path::new("/mnt/SDCARD/Apps/Slot"));
    assert_eq!(
        l.boot_hook(),
        Path::new("/mnt/SDCARD/System/starts/slot-boot.sh")
    );
    assert_eq!(
        l.boot_script(),
        Path::new("/mnt/SDCARD/Apps/Slot/boot/slot-boot.sh")
    );
    assert!(!l.is_flat());
}

#[test]
fn flat_is_the_layout_slot_always_had() {
    let l = Layout::flat(Path::new("/card"));
    assert_eq!(l.data(), Path::new("/card"));
    assert_eq!(l.roms(Platform::Gba), Path::new("/card/Games/GBA"));
    assert_eq!(l.app(), Path::new("/card/System"));
    assert_eq!(l.bios_dir(), Path::new("/card/BIOS"));
    assert!(l.is_flat());
}

#[test]
fn the_bios_comes_from_the_first_folder_that_holds_one() {
    let d = tempfile::tempdir().unwrap();
    let l = Layout::stockmix(d.path(), &d.path().join("Apps/Slot"));
    assert_eq!(
        l.bios_dir(),
        d.path().join("Slot/BIOS"),
        "with no bios anywhere, slot's own folder"
    );
    let retroarch = d.path().join("RetroArch/.retroarch/system");
    fs::create_dir_all(&retroarch).unwrap();
    fs::write(retroarch.join("gba_bios.bin"), b"x").unwrap();
    assert_eq!(l.bios_dir(), retroarch);
    fs::create_dir_all(d.path().join("BIOS")).unwrap();
    fs::write(d.path().join("BIOS/gba_bios.bin"), b"x").unwrap();
    assert_eq!(l.bios_dir(), d.path().join("BIOS"));
}

#[test]
fn a_root_with_nothing_installed_is_flat() {
    assert_eq!(
        Layout::for_root(Path::new("/nowhere")),
        Layout::flat(Path::new("/nowhere"))
    );
}

#[test]
fn the_stock_rom_folders_feed_the_shelf_and_nothing_else_does() {
    let d = tempfile::tempdir().unwrap();
    let l = Layout::stockmix(d.path(), &d.path().join("Apps/Slot"));
    let gba = d.path().join("Roms/GBA");
    fs::create_dir_all(gba.join("Hacks")).unwrap();
    fs::create_dir_all(gba.join(".mgba_libretro")).unwrap();
    fs::write(gba.join("Emerald.gba"), vec![0u8; 0x100]).unwrap();
    fs::write(gba.join("Zipped.zip"), b"PK").unwrap();
    fs::write(gba.join("GBA_cache7.db"), b"sqlite").unwrap();
    fs::write(gba.join("Hacks/Deep.gba"), vec![0u8; 0x100]).unwrap();
    fs::create_dir_all(d.path().join("Roms/GB")).unwrap();
    fs::write(d.path().join("Roms/GB/Tetris.gb"), vec![0u8; 0x150]).unwrap();
    let mut roms: Vec<PathBuf> = scan_in(&l).unwrap().into_iter().map(|c| c.rom).collect();
    roms.sort();
    let mut want = vec![gba.join("Emerald.gba"), d.path().join("Roms/GB/Tetris.gb")];
    want.sort();
    assert_eq!(roms, want);
}
