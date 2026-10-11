use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use slot::label_fetch::{curl_got, wget_got, Found, Getter, Got, LabelFetch};
use slot_store::{crc32, Cart, Platform};

struct Fake {
    have: Vec<String>,
    offline: bool,
    asked: Mutex<Vec<String>>,
}

impl Fake {
    fn with(have: &[String]) -> Arc<Fake> {
        Arc::new(Fake {
            have: have.to_vec(),
            offline: false,
            asked: Mutex::new(Vec::new()),
        })
    }

    fn offline() -> Arc<Fake> {
        Arc::new(Fake {
            have: Vec::new(),
            offline: true,
            asked: Mutex::new(Vec::new()),
        })
    }

    fn asked(&self) -> usize {
        self.asked.lock().unwrap().len()
    }
}

impl Getter for Fake {
    fn get(&self, url: &str, to: &Path) -> Got {
        self.asked.lock().unwrap().push(url.to_string());
        if self.offline {
            return Got::Failed;
        }
        if self.have.iter().any(|u| u == url) {
            std::fs::write(to, b"png").unwrap();
            Got::Saved
        } else {
            Got::Missing
        }
    }
}

fn cart(sd: &Path, platform: Platform, stem: &str, bytes: &[u8]) -> Cart {
    let dir = sd.join("Roms").join(platform.dir_name());
    std::fs::create_dir_all(&dir).unwrap();
    let rom = dir.join(format!("{stem}.bin"));
    std::fs::write(&rom, bytes).unwrap();
    Cart {
        platform,
        stem: stem.to_string(),
        rom,
        label: None,
        title: String::new(),
        code: String::new(),
        shell: None,
    }
}

fn url(dir: &str, bytes: &[u8]) -> String {
    format!(
        "https://art.slot-cfw.fyi/{dir}/support-texture/{:08X}.png",
        crc32(bytes)
    )
}

fn finish(f: &LabelFetch) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !f.finished() {
        assert!(Instant::now() < deadline, "the fetch never finished");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn a_found_label_lands_in_slots_own_labels_folder() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("Slot");
    let emerald = cart(d.path(), Platform::Gba, "Emerald", b"emerald rom");
    let getter = Fake::with(&[url("GBA", b"emerald rom")]);
    let f = LabelFetch::start(&root, vec![emerald], getter);
    finish(&f);
    let want = root.join("Labels/GBA/Emerald.png");
    assert_eq!(
        f.found(),
        vec![Found {
            platform: Platform::Gba,
            stem: "Emerald".into(),
            label: want.clone()
        }]
    );
    assert_eq!(std::fs::read(&want).unwrap(), b"png");
    assert_eq!((f.done(), f.total()), (1, 1));
    assert!(!f.offline());
}

#[test]
fn a_gb_label_filed_under_gbc_is_still_found() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("Slot");
    let tetris = cart(d.path(), Platform::Gb, "Tetris DX", b"tetris dx rom");
    let getter = Fake::with(&[url("GBC", b"tetris dx rom")]);
    let f = LabelFetch::start(&root, vec![tetris], getter.clone());
    finish(&f);
    assert_eq!(f.found().len(), 1);
    assert_eq!(getter.asked(), 2, "GB first, then GBC");
    assert!(root.join("Labels/GB/Tetris DX.png").is_file());
}

#[test]
fn a_rom_with_no_label_is_left_alone_and_leaves_nothing_behind() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("Slot");
    let hack = cart(d.path(), Platform::Gba, "Hack", b"patched rom");
    let f = LabelFetch::start(&root, vec![hack], Fake::with(&[]));
    finish(&f);
    assert!(f.found().is_empty());
    let left: Vec<PathBuf> = std::fs::read_dir(root.join("Labels/GBA"))
        .map(|r| r.filter_map(|e| e.ok()).map(|e| e.path()).collect())
        .unwrap_or_default();
    assert!(left.is_empty(), "left behind {left:?}");
}

#[test]
fn three_carts_in_a_row_that_cannot_reach_the_server_stop_the_run() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("Slot");
    let carts: Vec<Cart> = (0..6)
        .map(|i| {
            cart(
                d.path(),
                Platform::Gba,
                &format!("Game {i}"),
                &[i as u8; 64],
            )
        })
        .collect();
    let getter = Fake::offline();
    let f = LabelFetch::start(&root, carts, getter.clone());
    finish(&f);
    assert!(f.offline());
    assert_eq!(getter.asked(), 3, "kept knocking with no network");
    assert_eq!(
        f.done(),
        f.total(),
        "an aborted run still reads as finished"
    );
}

#[test]
fn curl_and_wget_answers_mean_saved_missing_or_failed() {
    assert_eq!(curl_got(Some(0)), Got::Saved);
    assert_eq!(curl_got(Some(22)), Got::Missing);
    assert_eq!(curl_got(Some(6)), Got::Failed);
    assert_eq!(curl_got(None), Got::Failed);
    assert_eq!(wget_got(true, ""), Got::Saved);
    assert_eq!(
        wget_got(false, "wget: server returned error: HTTP/1.1 404 Not Found"),
        Got::Missing
    );
    assert_eq!(
        wget_got(false, "wget: bad address 'art.slot-cfw.fyi'"),
        Got::Failed
    );
}

#[test]
fn wget_is_asked_whenever_curl_neither_saved_nor_heard_a_404() {
    use slot::label_fetch::settle;
    assert_eq!(settle(Some(Got::Failed), || Got::Saved), Got::Saved);
    assert_eq!(settle(Some(Got::Failed), || Got::Missing), Got::Missing);
    assert_eq!(settle(None, || Got::Saved), Got::Saved, "no curl at all");
    assert_eq!(
        settle(Some(Got::Saved), || panic!("asked wget anyway")),
        Got::Saved
    );
    assert_eq!(
        settle(Some(Got::Missing), || panic!("asked wget anyway")),
        Got::Missing
    );
}
