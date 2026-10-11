mod common;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use common::{app_playing_in, tmp_root_with_carts, CLOCK_IS_SET};
use slot::app::Phase;
use slot_input::Action;
use slot_power::{Battery, Charge, LedState, Platform, Power};

struct Suspending {
    root: PathBuf,
    suspends: Arc<AtomicUsize>,
    works: bool,
}

impl Platform for Suspending {
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
        &self.root
    }
    fn now(&self) -> i64 {
        CLOCK_IS_SET
    }
    fn set_clock(&mut self, _secs: i64) {}
    fn set_rumble(&mut self, _strength: u16) {}
    fn can_suspend(&self) -> bool {
        true
    }
    fn suspend(&mut self) -> bool {
        self.suspends.fetch_add(1, Ordering::Relaxed);
        self.works
    }
}

fn suspending(root: &Path, works: bool) -> (Power, Arc<AtomicUsize>) {
    let suspends = Arc::new(AtomicUsize::new(0));
    let platform = Suspending {
        root: root.to_path_buf(),
        suspends: suspends.clone(),
        works,
    };
    (
        Power::new(Box::new(platform), Duration::from_secs(3)),
        suspends,
    )
}

#[test]
fn a_doze_that_runs_out_suspends_and_wakes_rather_than_powering_off() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    let (power, suspends) = suspending(d.path(), true);
    a.set_power(power);
    a.apply(Action::PowerTap);
    assert!(matches!(a.phase(), Phase::Doze { .. }));
    a.on_doze_timeout();
    assert_eq!(suspends.load(Ordering::Relaxed), 1);
    assert!(!a.powering_off(), "a brick that can sleep was switched off");
    assert!(
        matches!(a.phase(), Phase::Playing { .. }),
        "the wake did not bring the game back: {:?}",
        a.phase()
    );
}

#[test]
fn the_power_press_that_woke_the_brick_does_not_doze_it_again() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    let (power, _) = suspending(d.path(), true);
    a.set_power(power);
    a.apply_at(Action::PowerTap, 1_000);
    assert!(matches!(a.phase(), Phase::Doze { .. }));
    a.on_doze_timeout();
    a.apply_at(Action::PowerPress, 1_100);
    a.apply_at(Action::PowerTap, 1_300);
    assert!(
        matches!(a.phase(), Phase::Playing { .. }),
        "the waking press dozed it straight back: {:?}",
        a.phase()
    );
    a.apply_at(Action::PowerPress, 4_000);
    a.apply_at(Action::PowerTap, 4_200);
    assert!(
        matches!(a.phase(), Phase::Doze { .. }),
        "a later tap no longer dozes"
    );
}

#[test]
fn a_suspend_that_fails_falls_back_to_powering_off() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    let (power, suspends) = suspending(d.path(), false);
    a.set_power(power);
    a.apply(Action::PowerTap);
    a.on_doze_timeout();
    assert_eq!(suspends.load(Ordering::Relaxed), 1);
    assert!(a.powering_off());
}
