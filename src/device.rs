use usbd_human_interface_device::usb_class::prelude::*;
use usbd_human_interface_device::UsbHidError;
use core::default::Default;
use fugit::ExtU32;
use usb_device::bus::UsbBus;
use usb_device::class_prelude::UsbBusAllocator;
use packed_struct::prelude::*;
use defmt::{
    error,
    unwrap,
    Format,
};

#[rustfmt::skip]
pub const JOYSTICK_DESCRIPTOR: &[u8] = &[
    0x05, 0x01, // Usage Page (Generic Desktop)
    0x09, 0x05, // Usage (Gamepad 0x05, Joystick 0x04)

    0xA1, 0x01, // Collection (Application)
        0x09, 0x01, //   Usage Page (Pointer)
        0xA1, 0x00, //   Collection (Physical)
            0x09, 0x30, //     Usage (X)
            0x09, 0x31, //     Usage (Y)
            0x09, 0x32, //     Usage (Z) Trigger (Not used)
            0x09, 0x33, //     Usage (RX) - Second joystick
            0x09, 0x34, //     Usage (RY) - Second joystick
            0x09, 0x35, //     Usage (RZ) Trigger (Not used)
            0x15, 0x81, //     Logical Minimum (-127)
            0x25, 0x7f, //     Logical Maximum (127)
            0x75, 0x08, //     Report Size
            0x95, 0x06, //     Report count
            0x81, 0x02, //     Input (Data, Variable, Absolute)
        0xC0,       //   End Collection

        0x05, 0x09, //   Usage Page (Button)
        0x19, 0x01, //   Usage Minimum (0)
        0x29, 0x10, //   Usage Maximum (16)
        0x15, 0x00, //   Logical Minimum (0)
        0x25, 0x01, //   Logical Maximum (1)
        0x75, 0x01, //   Report Size (1)
        0x95, 0x10, //   Report Count (16)
        0x81, 0x02, //   Input (Data, Variable, Absolute)
    0xC0,       // End Collection

    /* TODO: 16 bit joy resolution
    0x05, 0x01, // Usage Page (Generic Desktop)
    0x09, 0x04, // Usage (Joystick)

    0xA1, 0x01, // Collection (Application)
    0x09, 0x01, // Usage Pointer
        0xA1, 0x00, // Collection (Physical)
            0x05, 0x09, // Usage Button 
            0x19, 0x01, // Button 0
            0x29, 0x08, // Button 8
            0x15, 0x00, // Logical Min 
            0x25, 0x01, // Logical Max
            0x95, 0x08, // Report Count
            0x75, 0x01, // Report Size in bits
            0x81, 0x02, // Input 

            0x05, 0x01, // Usage Page
            0x09, 0x30, //     Usage (X)
            0x09, 0x31, //     Usage (Y)
            0x09, 0x32, //     Usage (X) - Second joystick
            0x09, 0x33, //     Usage (Y) - Second joystick
            0x16, 0x80, 0x01, //     Logical Minimum (-32768)
            0x26, 0x7F, 0xFF, //     Logical Maximum (32767)
            0x75, 0x10, //     Report Size (16)
            0x95, 0x04, //     Report count (4)
            0x81, 0x02, //     Input (Data, Variable, Absolute)
        0xC0,       //   End Collection Physical,
    0xC0,       //   End Collection Application,
    */
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default, PackedStruct)]
#[derive(Format)]
#[packed_struct(endian = "lsb", size_bytes = "8")]
pub struct JoystickReport {
    #[packed_field]
    pub ly: i8,
    #[packed_field]
    pub lx: i8,
    #[packed_field]
    pub lz: i8,
    #[packed_field]
    pub ry: i8,
    #[packed_field]
    pub rx: i8,
    #[packed_field]
    pub rz: i8,
    #[packed_field]
    pub buttons: u16,
}

pub struct Joystick<'a, B: UsbBus> {
    interface: Interface<'a, B, InBytes8, OutNone, ReportSingle>,
}

impl<'a, B: UsbBus> Joystick<'a, B> {
    pub fn write_report(&mut self, report: &JoystickReport) -> Result<(), UsbHidError> {
        let data = report.pack().map_err(|_| {
            error!("Error packing JoystickReport");
            UsbHidError::SerializationError
        })?;
        self.interface
            .write_report(&data)
            .map(|_| ())
            .map_err(UsbHidError::from)
    }
}

impl<'a, B: UsbBus> DeviceClass<'a> for Joystick<'a, B> {
    type I = Interface<'a, B, InBytes8, OutNone, ReportSingle>;

    fn interface(&mut self) -> &mut Self::I {
        &mut self.interface
    }

    fn reset(&mut self) {}

    fn tick(&mut self) -> Result<(), UsbHidError> {
        Ok(())
    }
}

pub struct JoystickConfig<'a> {
    interface: InterfaceConfig<'a, InBytes8, OutNone, ReportSingle>,
}

impl<'a> Default for JoystickConfig<'a> {
    #[must_use]
    fn default() -> Self {
        Self::new(
            unwrap!(unwrap!(InterfaceBuilder::new(JOYSTICK_DESCRIPTOR))
                .boot_device(InterfaceProtocol::None)
                .description("Joystick")
                .in_endpoint(10.millis()))
            .without_out_endpoint()
            .build(),
        )
    }
}

impl<'a> JoystickConfig<'a> {
    #[must_use]
    pub fn new(interface: InterfaceConfig<'a, InBytes8, OutNone, ReportSingle>) -> Self {
        Self { interface }
    }
}

impl<'a, B: UsbBus + 'a> UsbAllocatable<'a, B> for JoystickConfig<'a> {
    type Allocated = Joystick<'a, B>;

    fn allocate(self, usb_alloc: &'a UsbBusAllocator<B>) -> Self::Allocated {
        Self::Allocated {
            interface: Interface::new(usb_alloc, self.interface),
        }
    }
}

