mod device;
mod mixer;
mod motor;
mod power;
mod sim;

pub use device::{
    backlight_value, has_bit, led_hex, motor_change, motor_voltage, read_only, read_only_in,
    record_first_frame, trace_first_frame, uptime_seconds, DevicePlatform, BACKLIGHT,
    LED_SCALE_F1F2, LED_STATIC, MOTOR_MAX_UV, MOTOR_MIN_UV, RUMBLE_GPIO,
};
pub use mixer::{parse_values, Amixer, CliAmixer, HwVolume, LOUD};
pub use motor::Motor;
pub use power::Power;
pub use sim::SimPlatform;

use std::path::Path;
use std::time::Duration;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Charge {
    Unknown,
    Discharging,
    Charging,
    Full,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Battery {
    pub percent: u8,
    pub charge: Charge,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum LedState {
    Off,
    Running,
    Low,
    Charging,
    Charged,
}

pub trait Platform: Send {
    fn set_backlight(&mut self, step: u8);
    fn battery(&self) -> Option<Battery>;
    fn charge(&self) -> Charge;
    fn set_led(&mut self, state: LedState);
    fn poweroff(&mut self) -> !;
    fn restart(&mut self) -> !;
    fn root(&self) -> &Path;
    fn now(&self) -> i64;
    fn set_clock(&mut self, secs: i64);
    fn set_rumble(&mut self, strength: u16);

    fn headphones(&self) -> bool {
        false
    }

    fn release(&mut self) {}

    fn card_read_only(&self) -> bool {
        false
    }
}

pub trait LidPolicy {
    fn on_close(&mut self);
    fn on_open(&mut self);
    fn timeout(&self) -> Duration;
}
