use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use slot_power::{Battery, Charge, LedState, Platform, Power};

struct Counting(Arc<AtomicUsize>, PathBuf);

impl Platform for Counting {
    fn set_backlight(&mut self, _step: u8) {}
    fn battery(&self) -> Option<Battery> {
        None
    }
    fn charge(&self) -> Charge {
        Charge::Unknown
    }
    fn set_led(&mut self, _state: LedState) {}
    fn poweroff(&mut self) -> ! {
        unreachable!()
    }
    fn restart(&mut self) -> ! {
        unreachable!()
    }
    fn root(&self) -> &Path {
        &self.1
    }
    fn now(&self) -> i64 {
        0
    }
    fn set_clock(&mut self, _secs: i64) {}
    fn set_rumble(&mut self, _strength: u16) {}
    fn release(&mut self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn release_reaches_the_platform() {
    let n = Arc::new(AtomicUsize::new(0));
    let mut p = Power::new(
        Box::new(Counting(n.clone(), PathBuf::new())),
        Duration::from_secs(1),
    );
    p.release();
    assert_eq!(n.load(Ordering::Relaxed), 1);
}
