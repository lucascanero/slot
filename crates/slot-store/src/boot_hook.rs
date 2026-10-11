use std::fs;
use std::io;
use std::path::PathBuf;

use crate::layout::Layout;

pub struct BootHook {
    script: PathBuf,
    target: PathBuf,
}

impl BootHook {
    pub fn for_layout(layout: &Layout) -> BootHook {
        BootHook {
            script: layout.boot_script(),
            target: layout.boot_hook().to_path_buf(),
        }
    }

    pub fn installed(&self) -> bool {
        self.target.is_file()
    }

    pub fn set(&self, on: bool) -> io::Result<()> {
        if !on {
            return match fs::remove_file(&self.target) {
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
                other => other,
            };
        }
        if let Some(dir) = self.target.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::copy(&self.script, &self.target)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&self.target, fs::Permissions::from_mode(0o755))?;
        }
        Ok(())
    }
}
