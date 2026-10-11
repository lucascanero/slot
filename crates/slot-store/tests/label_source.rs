use std::fs;

use slot_store::{crc32, crc32_file, label_urls, Platform};

#[test]
fn the_checksum_is_the_standard_crc32() {
    assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    assert_eq!(crc32(b""), 0);
}

#[test]
fn a_rom_file_is_checksummed_whole() {
    let d = tempfile::tempdir().unwrap();
    let rom = d.path().join("game.gba");
    let bytes: Vec<u8> = (0..300_000u32).map(|i| (i * 7) as u8).collect();
    fs::write(&rom, &bytes).unwrap();
    assert_eq!(crc32_file(&rom).unwrap(), crc32(&bytes));
}

#[test]
fn labels_are_looked_up_by_platform_and_checksum() {
    assert_eq!(
        label_urls(Platform::Gba, 0x0002_06C0),
        ["https://art.slot-cfw.fyi/GBA/support-texture/000206C0.png"]
    );
    assert_eq!(
        label_urls(Platform::Gb, 0xABCD_EF01),
        [
            "https://art.slot-cfw.fyi/GB/support-texture/ABCDEF01.png",
            "https://art.slot-cfw.fyi/GBC/support-texture/ABCDEF01.png"
        ]
    );
    assert_eq!(
        label_urls(Platform::Gbc, 1),
        [
            "https://art.slot-cfw.fyi/GBC/support-texture/00000001.png",
            "https://art.slot-cfw.fyi/GB/support-texture/00000001.png"
        ]
    );
}
