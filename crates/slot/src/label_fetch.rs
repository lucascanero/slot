use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use slot_store::{crc32_file, label_urls, Cart, Platform};

const GIVE_UP_AFTER: usize = 3;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Got {
    Saved,
    Missing,
    Failed,
}

pub trait Getter: Send + Sync {
    fn get(&self, url: &str, to: &Path) -> Got;
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Found {
    pub platform: Platform,
    pub stem: String,
    pub label: PathBuf,
}

pub struct LabelFetch {
    done: Arc<AtomicUsize>,
    total: usize,
    found: Arc<Mutex<Vec<Found>>>,
    offline: Arc<AtomicBool>,
}

impl LabelFetch {
    pub fn start(root: &Path, carts: Vec<Cart>, getter: Arc<dyn Getter>) -> Self {
        let total = carts.len();
        let done = Arc::new(AtomicUsize::new(0));
        let found = Arc::new(Mutex::new(Vec::new()));
        let offline = Arc::new(AtomicBool::new(false));
        let (root, worker_done, worker_found, worker_offline) = (
            root.to_path_buf(),
            done.clone(),
            found.clone(),
            offline.clone(),
        );
        let spawned = thread::Builder::new()
            .name("slot-label-fetch".into())
            .spawn(move || {
                let mut failing = 0;
                for cart in &carts {
                    if failing >= GIVE_UP_AFTER {
                        worker_offline.store(true, Ordering::Release);
                        break;
                    }
                    match fetch_one(&root, cart, &*getter) {
                        Got::Saved => {
                            failing = 0;
                            worker_found
                                .lock()
                                .unwrap_or_else(|e| e.into_inner())
                                .push(Found {
                                    platform: cart.platform,
                                    stem: cart.stem.clone(),
                                    label: label_path(&root, cart),
                                });
                        }
                        Got::Missing => failing = 0,
                        Got::Failed => failing += 1,
                    }
                    worker_done.fetch_add(1, Ordering::Release);
                }
                if failing >= GIVE_UP_AFTER {
                    worker_offline.store(true, Ordering::Release);
                }
                worker_done.store(total, Ordering::Release);
            });
        if let Err(e) = spawned {
            eprintln!("slot: labels: fetch thread failed to start: {e}");
            done.store(total, Ordering::Release);
        }
        LabelFetch {
            done,
            total,
            found,
            offline,
        }
    }

    pub fn done(&self) -> usize {
        self.done.load(Ordering::Acquire).min(self.total)
    }

    pub fn total(&self) -> usize {
        self.total
    }

    pub fn finished(&self) -> bool {
        self.done() == self.total
    }

    pub fn offline(&self) -> bool {
        self.offline.load(Ordering::Acquire)
    }

    pub fn found(&self) -> Vec<Found> {
        self.found.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

fn label_path(root: &Path, cart: &Cart) -> PathBuf {
    root.join("Labels")
        .join(cart.platform.dir_name())
        .join(format!("{}.png", cart.stem))
}

fn fetch_one(root: &Path, cart: &Cart, getter: &dyn Getter) -> Got {
    let crc = match crc32_file(&cart.rom) {
        Ok(crc) => crc,
        Err(e) => {
            eprintln!("slot: labels: {}: {e}", cart.rom.display());
            return Got::Missing;
        }
    };
    let dest = label_path(root, cart);
    let Some(dir) = dest.parent() else {
        return Got::Missing;
    };
    if let Err(e) = std::fs::create_dir_all(dir) {
        eprintln!("slot: labels: {}: {e}", dir.display());
        return Got::Failed;
    }
    let part = dir.join(format!(".{}.part", cart.stem));
    for url in label_urls(cart.platform, crc) {
        match getter.get(&url, &part) {
            Got::Saved => {
                return match std::fs::rename(&part, &dest) {
                    Ok(()) => {
                        eprintln!("slot: labels: {} <- {url}", cart.stem);
                        Got::Saved
                    }
                    Err(e) => {
                        eprintln!("slot: labels: {}: {e}", dest.display());
                        let _ = std::fs::remove_file(&part);
                        Got::Missing
                    }
                };
            }
            Got::Missing => {
                let _ = std::fs::remove_file(&part);
            }
            Got::Failed => {
                let _ = std::fs::remove_file(&part);
                return Got::Failed;
            }
        }
    }
    eprintln!("slot: labels: none for {} ({crc:08X})", cart.stem);
    Got::Missing
}

pub fn curl_got(code: Option<i32>) -> Got {
    match code {
        Some(0) => Got::Saved,
        Some(22) => Got::Missing,
        _ => Got::Failed,
    }
}

pub fn wget_got(success: bool, stderr: &str) -> Got {
    if success {
        Got::Saved
    } else if stderr.contains("404") {
        Got::Missing
    } else {
        Got::Failed
    }
}

pub fn settle(curl: Option<Got>, wget: impl FnOnce() -> Got) -> Got {
    match curl {
        Some(Got::Saved) => Got::Saved,
        Some(Got::Missing) => Got::Missing,
        _ => wget(),
    }
}

pub struct CliGetter;

impl Getter for CliGetter {
    fn get(&self, url: &str, to: &Path) -> Got {
        let curl = Command::new("curl")
            .args(["-fsSL", "--connect-timeout", "5", "--max-time", "30", "-o"])
            .arg(to)
            .arg(url)
            .status()
            .ok()
            .map(|status| curl_got(status.code()));
        settle(curl, || {
            match Command::new("wget")
                .args(["-q", "-T", "20", "-O"])
                .arg(to)
                .arg(url)
                .output()
            {
                Ok(out) => wget_got(out.status.success(), &String::from_utf8_lossy(&out.stderr)),
                Err(e) => {
                    eprintln!("slot: labels: wget did not run: {e}");
                    Got::Failed
                }
            }
        })
    }
}
