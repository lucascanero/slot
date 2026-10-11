use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;

use slot_power::{parse_values, Amixer, HwVolume};

struct Fake(RefCell<HashMap<String, String>>);

impl Fake {
    fn with(controls: &[(&str, &str)]) -> Fake {
        Fake(RefCell::new(
            controls
                .iter()
                .map(|(c, v)| (c.to_string(), v.to_string()))
                .collect(),
        ))
    }

    fn get(&self, control: &str) -> Option<String> {
        self.0.borrow().get(control).cloned()
    }
}

impl Amixer for Fake {
    fn cget(&self, control: &str) -> Option<String> {
        self.get(control)
    }

    fn cset(&self, control: &str, values: &str) -> bool {
        let mut map = self.0.borrow_mut();
        match map.get_mut(control) {
            Some(v) => {
                *v = values.to_string();
                true
            }
            None => false,
        }
    }
}

#[test]
fn amixer_values_are_read_off_the_cget_line() {
    let out = "numid=5,iface=MIXER,name='DAC volume'\n  ; type=INTEGER,access=rw------,values=2,min=0,max=255,step=0\n  : values=160,160\n";
    assert_eq!(parse_values(out).as_deref(), Some("160,160"));
    assert_eq!(parse_values("amixer: Cannot find the given element"), None);
}

#[test]
fn the_hardware_is_turned_full_up_and_put_back_after() {
    let d = tempfile::tempdir().unwrap();
    fs::create_dir_all(d.path().join("class/speaker")).unwrap();
    fs::write(d.path().join("class/speaker/mute"), "1\n").unwrap();
    let fake = Fake::with(&[("digital volume", "40"), ("DAC volume", "100,100")]);
    let v = HwVolume::take(&fake, d.path());
    assert_eq!(fake.get("digital volume").as_deref(), Some("0"));
    assert_eq!(fake.get("DAC volume").as_deref(), Some("160"));
    let mute = || fs::read_to_string(d.path().join("class/speaker/mute")).unwrap();
    assert_eq!(mute().trim(), "0");
    v.restore(&fake);
    assert_eq!(fake.get("digital volume").as_deref(), Some("40"));
    assert_eq!(fake.get("DAC volume").as_deref(), Some("100,100"));
    assert_eq!(mute().trim(), "1");
}

#[test]
fn a_control_the_card_lacks_is_left_alone() {
    let d = tempfile::tempdir().unwrap();
    let fake = Fake::with(&[("DAC volume", "160")]);
    let v = HwVolume::take(&fake, d.path());
    v.restore(&fake);
    assert_eq!(fake.get("digital volume"), None);
    assert_eq!(fake.get("DAC volume").as_deref(), Some("160"));
}
