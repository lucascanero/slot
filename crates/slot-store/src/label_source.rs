use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use crate::platform::Platform;

pub const LABEL_ART: &str = "https://art.slot-cfw.fyi/";

const TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut n = 0;
    while n < 256 {
        let mut c = n as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 == 1 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        table[n] = c;
        n += 1;
    }
    table
};

fn update(crc: u32, bytes: &[u8]) -> u32 {
    bytes.iter().fold(crc, |c, b| {
        TABLE[((c ^ u32::from(*b)) & 0xFF) as usize] ^ (c >> 8)
    })
}

pub fn crc32(bytes: &[u8]) -> u32 {
    !update(!0, bytes)
}

pub fn crc32_file(path: &Path) -> io::Result<u32> {
    let mut file = File::open(path)?;
    let mut buf = vec![0u8; 1 << 16];
    let mut crc = !0;
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            return Ok(!crc);
        }
        crc = update(crc, &buf[..n]);
    }
}

pub fn label_urls(platform: Platform, crc: u32) -> Vec<String> {
    let dirs: &[&str] = match platform {
        Platform::Gba => &["GBA"],
        Platform::Gb => &["GB", "GBC"],
        Platform::Gbc => &["GBC", "GB"],
    };
    dirs.iter()
        .map(|dir| format!("{LABEL_ART}{dir}/support-texture/{crc:08X}.png"))
        .collect()
}
