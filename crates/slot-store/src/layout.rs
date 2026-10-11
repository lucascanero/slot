use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::platform::Platform;

pub const DATA_DIR: &str = "Slot";
pub const BIOS_FILE: &str = "gba_bios.bin";
pub const BOOT_HOOK_DIR: &str = "System/starts";
pub const BOOT_HOOK_NAME: &str = "slot-boot.sh";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    data: PathBuf,
    roms: PathBuf,
    art: Option<PathBuf>,
    app: PathBuf,
    bios: Vec<PathBuf>,
    boot_hook: PathBuf,
    flat: bool,
}

static INSTALLED: OnceLock<Layout> = OnceLock::new();

impl Layout {
    pub fn flat(root: &Path) -> Layout {
        Layout {
            data: root.to_path_buf(),
            roms: root.join("Games"),
            art: None,
            app: root.join("System"),
            bios: vec![root.join("BIOS")],
            boot_hook: root.join(BOOT_HOOK_DIR).join(BOOT_HOOK_NAME),
            flat: true,
        }
    }

    pub fn stockmix(sd: &Path, app: &Path) -> Layout {
        let data = sd.join(DATA_DIR);
        Layout {
            roms: sd.join("Roms"),
            art: Some(sd.join("Imgs")),
            app: app.to_path_buf(),
            bios: vec![
                sd.join("BIOS"),
                sd.join("Bios"),
                sd.join("RetroArch/.retroarch/system"),
                data.join("BIOS"),
            ],
            boot_hook: sd.join(BOOT_HOOK_DIR).join(BOOT_HOOK_NAME),
            data,
            flat: false,
        }
    }

    pub fn install(self) -> bool {
        INSTALLED.set(self).is_ok()
    }

    pub fn installed() -> Option<&'static Layout> {
        INSTALLED.get()
    }

    pub fn for_root(root: &Path) -> Layout {
        INSTALLED
            .get()
            .filter(|l| l.data() == root)
            .cloned()
            .unwrap_or_else(|| Layout::flat(root))
    }

    pub fn is_flat(&self) -> bool {
        self.flat
    }

    pub fn data(&self) -> &Path {
        &self.data
    }

    pub fn roms(&self, platform: Platform) -> PathBuf {
        self.roms.join(platform.dir_name())
    }

    pub fn art(&self, platform: Platform) -> Option<PathBuf> {
        self.art.as_ref().map(|d| d.join(platform.dir_name()))
    }

    pub fn app(&self) -> &Path {
        &self.app
    }

    pub fn bios_dir(&self) -> PathBuf {
        self.bios
            .iter()
            .find(|dir| dir.join(BIOS_FILE).is_file())
            .or(self.bios.last())
            .cloned()
            .unwrap_or_else(|| self.data.join("BIOS"))
    }

    pub fn boot_hook(&self) -> &Path {
        &self.boot_hook
    }

    pub fn boot_script(&self) -> PathBuf {
        self.app.join("boot").join(BOOT_HOOK_NAME)
    }
}

pub fn app_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("SLOT_APP") {
        return PathBuf::from(dir);
    }
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}
