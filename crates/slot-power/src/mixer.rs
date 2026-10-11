use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const LOUD: [(&str, &str); 2] = [("digital volume", "0"), ("DAC volume", "160")];

pub trait Amixer {
    fn cget(&self, control: &str) -> Option<String>;
    fn cset(&self, control: &str, values: &str) -> bool;
}

pub struct CliAmixer;

impl Amixer for CliAmixer {
    fn cget(&self, control: &str) -> Option<String> {
        let out = Command::new("amixer")
            .args(["cget", &format!("name={control}")])
            .output()
            .ok()?;
        parse_values(&String::from_utf8_lossy(&out.stdout))
    }

    fn cset(&self, control: &str, values: &str) -> bool {
        Command::new("amixer")
            .args(["-q", "cset", &format!("name={control}"), values])
            .status()
            .is_ok_and(|s| s.success())
    }
}

pub fn parse_values(text: &str) -> Option<String> {
    text.lines()
        .find_map(|l| l.trim().strip_prefix(": values="))
        .map(|v| v.trim().to_string())
}

pub struct HwVolume {
    saved: Vec<(String, String)>,
    speaker: Option<(PathBuf, String)>,
}

impl HwVolume {
    pub fn take(mixer: &dyn Amixer, sysfs: &Path) -> HwVolume {
        let mut saved = Vec::new();
        for (control, loud) in LOUD {
            if let Some(old) = mixer.cget(control) {
                if mixer.cset(control, loud) {
                    saved.push((control.to_string(), old));
                }
            }
        }
        let mute = sysfs.join("class/speaker/mute");
        let speaker = fs::read_to_string(&mute)
            .ok()
            .map(|old| (mute, old.trim().to_string()));
        if let Some((path, _)) = &speaker {
            let _ = fs::write(path, "0\n");
        }
        eprintln!("slot: hardware volume taken: {saved:?}");
        HwVolume { saved, speaker }
    }

    pub fn restore(&self, mixer: &dyn Amixer) {
        for (control, old) in self.saved.iter().rev() {
            mixer.cset(control, old);
        }
        if let Some((path, old)) = &self.speaker {
            let _ = fs::write(path, format!("{old}\n"));
        }
    }
}
