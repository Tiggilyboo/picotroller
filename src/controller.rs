use core::fmt::Debug;
use crate::device::JoystickReport;

const ADC_MAX_VALUE_3V3: u16 = 4095;

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

impl Controller {
    pub fn hid_report(&self, report: &mut JoystickReport) {
        report.lx = scale_i8(self.joy_l.x, ADC_MAX_VALUE_3V3);
        report.ly = scale_i8(self.joy_l.y, ADC_MAX_VALUE_3V3);
        report.rx = scale_i8(self.joy_r.x, ADC_MAX_VALUE_3V3);
        report.ry = scale_i8(self.joy_r.y, ADC_MAX_VALUE_3V3);
        report.buttons = 0b0000_0000;
        if self.joy_l.button {
            report.buttons |= 0b0000_0010;
        }
        if self.joy_r.button {
            report.buttons |= 0b0000_0001;
        }
    }
}

fn scale_i8(value: u16, max: u16) -> i8 {
    (128f32 - ((value as f32 / max as f32) * 255.0f32)) as i8
}
