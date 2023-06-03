use core::fmt::Debug;
use crate::device::JoystickReport;

const ADC_MAX_VALUE_3V3: i32 = 4095;

#[derive(Debug)]
pub struct JoyState {
    pub button: bool,
    pub x: u16,
    pub y: u16,
}

impl Default for JoyState {
    fn default() -> Self {
        Self {
            button: false,
            x: 0,
            y: 0,
        }
    }
}

#[derive(Debug)]
pub struct Controller {
    pub joy_l: JoyState,
    pub joy_r: JoyState,
}

impl Default for Controller {
    fn default() -> Self {
        Self {
            joy_l: JoyState::default(),
            joy_r: JoyState::default(),
        }
    }
}

#[allow(unused)]
mod buttons {
    pub const BTN_SOUTH: u16 = 1 << 0;
    pub const BTN_EAST: u16 = 1 << 1;
    pub const BTN_C: u16 = 1 << 2;
    pub const BTN_NORTH: u16 = 1 << 3;
    pub const BTN_WEST: u16 = 1 << 4;
    pub const BTN_Z: u16 = 1 << 5;
    pub const BTN_TL: u16 = 1 << 6;
    pub const BTN_TR: u16 = 1 << 7;
    pub const BTN_TL2: u16 = 1 << 8;
    pub const BTN_TR2: u16 = 1 << 9;
    pub const BTN_SELECT: u16 = 1 << 10;
    pub const BTN_START: u16 = 1 << 11;
    pub const BTN_MODE: u16 = 1 << 12;
    pub const BTN_THUMBL: u16 = 1 << 13;
    pub const BTN_THUMBR: u16 = 1 << 14;
}

impl Controller {
    #[inline]
    pub fn hid_report(&self, report: &mut JoystickReport) {
        report.lx = scale_i8(self.joy_l.x);
        report.ly = scale_i8(self.joy_l.y);
        report.rx = scale_i8(self.joy_r.x);
        report.ry = scale_i8(self.joy_r.y);

        report.buttons = 0;
        if self.joy_l.button {
            report.buttons |= buttons::BTN_THUMBL;
        }
        if self.joy_r.button {
            report.buttons |= buttons::BTN_THUMBR;
        }
    }
}

/*
#[inline]
fn scale_i16(value: u16) -> i16 {
    let scaled_value = (value as i32 * 65535) / ADC_MAX_VALUE_3V3 as i32 - 32768;
    scaled_value as i16
}
*/

#[inline]
fn scale_i8(value: u16) -> i8 {
    let scaled_value = (value as i32 * 255) / ADC_MAX_VALUE_3V3 as i32 - 128;
    scaled_value as i8
}

