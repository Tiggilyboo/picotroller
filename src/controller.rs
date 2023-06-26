use crate::device::JoystickReport;
use core::fmt::Debug;

const ADC_MAX_VALUE_3V3: i32 = 4095;

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
    pub under_l: bool,
    pub under_r: bool,
    pub front_l: bool,
    pub front_r: bool,
    pub start: bool,
    pub select: bool,
}

impl Default for Controller {
    fn default() -> Self {
        Self {
            joy_l: JoyState::default(),
            joy_r: JoyState::default(),
            under_l: false,
            under_r: false,
            front_l: false,
            front_r: false,
            start: false,
            select: false,
        }
    }
}

impl Controller {
    #[inline]
    pub fn hid_report(&self, report: &mut JoystickReport) {
        report.lx = scale_i8(ADC_MAX_VALUE_3V3 as u16 - self.joy_l.x);
        report.ly = scale_i8(self.joy_l.y);
        report.rx = scale_i8(ADC_MAX_VALUE_3V3 as u16 - self.joy_r.x);
        report.ry = scale_i8(self.joy_r.y);

        report.buttons = 0;
        if self.joy_l.button {
            report.buttons |= buttons::BTN_THUMBL;
        }
        if self.joy_r.button {
            report.buttons |= buttons::BTN_THUMBR;
        }
        if self.under_l {
            report.buttons |= buttons::BTN_WEST;
        }
        if self.under_r {
            report.buttons |= buttons::BTN_NORTH;
        }
        if self.front_l {
            report.buttons |= buttons::BTN_EAST;
        }
        if self.front_r {
            report.buttons |= buttons::BTN_SOUTH;
        }
        if self.start {
            report.buttons |= buttons::BTN_START;
        }
        if self.start {
            report.buttons |= buttons::BTN_SELECT;
        }
    }
}

#[inline]
fn scale_i8(value: u16) -> i8 {
    let scaled_value = (value as i32 * 255) / ADC_MAX_VALUE_3V3 - 128;
    scaled_value as i8
}
