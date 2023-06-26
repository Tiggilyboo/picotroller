#![no_std]
#![no_main]

use defmt_rtt as _;

use core::cell::RefCell;

use bsp::hal;
use bsp::{entry, Pins};
use cortex_m::prelude::{_embedded_hal_adc_OneShot, _embedded_hal_timer_CountDown};
use critical_section::Mutex;
use fugit::ExtU32;
use hal::{
    clocks::init_clocks_and_plls, clocks::Clock, gpio, gpio::Interrupt, pac, pac::interrupt,
    pio::PIOExt, timer::Timer, watchdog::Watchdog, Sio,
};
use panic_halt as _;
use smart_leds::colors;
use smart_leds::{brightness, SmartLedsWrite};
use usb_device::class_prelude::UsbBusAllocator;
use usb_device::prelude::{UsbDeviceBuilder, UsbVidPid};
use usbd_human_interface_device::usb_class::UsbHidClassBuilder;
use usbd_human_interface_device::UsbHidError;
use waveshare_rp2040_zero as bsp;
use ws2812_pio::Ws2812;

mod controller;
use controller::*;

mod device;
use device::JoystickReport;

const USB_VENDOR: u16 = 0x045e;
const USB_PRODUCT: u16 = 0x028e;
const USB_MANUFACTURER: &'static str = "Nameless";
const USB_PRODUCT_NAME: &'static str = "Picotroller";
const USB_SERIALNUM: &'static str = "CTLPICO";

type ButtonPinThumbL = gpio::Pin<gpio::bank0::Gpio14, gpio::PullUpInput>;
type ButtonPinThumbR = gpio::Pin<gpio::bank0::Gpio8, gpio::PullUpInput>;
type ButtonPinUnderL = gpio::Pin<gpio::bank0::Gpio13, gpio::PullDownInput>;
type ButtonPinUnderR = gpio::Pin<gpio::bank0::Gpio9, gpio::PullDownInput>;
type ButtonPinFrontL = gpio::Pin<gpio::bank0::Gpio10, gpio::PullDownInput>;
type ButtonPinFrontR = gpio::Pin<gpio::bank0::Gpio11, gpio::PullDownInput>;
type ButtonPinStart = gpio::Pin<gpio::bank0::Gpio12, gpio::PullDownInput>;
type ButtonPinSelect = gpio::Pin<gpio::bank0::Gpio7, gpio::PullDownInput>;

static BUTTON_PIN_THUMB_L: Mutex<RefCell<Option<ButtonPinThumbL>>> = Mutex::new(RefCell::new(None));
static BUTTON_PIN_THUMB_R: Mutex<RefCell<Option<ButtonPinThumbR>>> = Mutex::new(RefCell::new(None));
static BUTTON_PIN_UNDER_L: Mutex<RefCell<Option<ButtonPinUnderL>>> = Mutex::new(RefCell::new(None));
static BUTTON_PIN_UNDER_R: Mutex<RefCell<Option<ButtonPinUnderR>>> = Mutex::new(RefCell::new(None));
static BUTTON_PIN_FRONT_L: Mutex<RefCell<Option<ButtonPinFrontL>>> = Mutex::new(RefCell::new(None));
static BUTTON_PIN_FRONT_R: Mutex<RefCell<Option<ButtonPinFrontR>>> = Mutex::new(RefCell::new(None));
static BUTTON_PIN_START: Mutex<RefCell<Option<ButtonPinStart>>> = Mutex::new(RefCell::new(None));
static BUTTON_PIN_SELECT: Mutex<RefCell<Option<ButtonPinSelect>>> = Mutex::new(RefCell::new(None));

// state
static BUTTON_THUMB_L: Mutex<RefCell<bool>> = Mutex::new(RefCell::new(false));
static BUTTON_THUMB_R: Mutex<RefCell<bool>> = Mutex::new(RefCell::new(false));
static BUTTON_UNDER_L: Mutex<RefCell<bool>> = Mutex::new(RefCell::new(false));
static BUTTON_UNDER_R: Mutex<RefCell<bool>> = Mutex::new(RefCell::new(false));
static BUTTON_FRONT_L: Mutex<RefCell<bool>> = Mutex::new(RefCell::new(false));
static BUTTON_FRONT_R: Mutex<RefCell<bool>> = Mutex::new(RefCell::new(false));
static BUTTON_START: Mutex<RefCell<bool>> = Mutex::new(RefCell::new(false));
static BUTTON_SELECT: Mutex<RefCell<bool>> = Mutex::new(RefCell::new(false));

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);

    let clocks = init_clocks_and_plls(
        bsp::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let sio = Sio::new(pac.SIO);
    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );
    let timer = Timer::new(pac.TIMER, &mut pac.RESETS);
    let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let mut led = Ws2812::new(
        pins.neopixel.into_mode(),
        &mut pio,
        sm0,
        clocks.peripheral_clock.freq(),
        timer.count_down(),
    );
    led.write(brightness(core::iter::once(colors::RED), 8))
        .unwrap();

    // START SETUP

    let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));
    let mut joy_hid = UsbHidClassBuilder::new()
        .add_device(device::JoystickConfig::default())
        .build(&usb_bus);

    let mut usb_device = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(USB_VENDOR, USB_PRODUCT))
        .manufacturer(USB_MANUFACTURER)
        .product(USB_PRODUCT_NAME)
        .serial_number(USB_SERIALNUM)
        .device_class(2)
        .build();

    led.write(brightness(core::iter::once(colors::RED), 6))
        .unwrap();

    // Setup joystick button interrupt pins
    {
        let l_joy_btn_pin = pins.gp14.into_mode();
        l_joy_btn_pin.set_interrupt_enabled(Interrupt::EdgeLow, true);
        l_joy_btn_pin.set_interrupt_enabled(Interrupt::EdgeHigh, true);
        critical_section::with(|cs| BUTTON_PIN_THUMB_L.borrow(cs).replace(Some(l_joy_btn_pin)));
    }
    {
        let r_joy_btn_pin = pins.gp8.into_mode();
        r_joy_btn_pin.set_interrupt_enabled(Interrupt::EdgeLow, true);
        r_joy_btn_pin.set_interrupt_enabled(Interrupt::EdgeHigh, true);
        critical_section::with(|cs| BUTTON_PIN_THUMB_R.borrow(cs).replace(Some(r_joy_btn_pin)));
    }
    // Setup under / front button interrupt pins
    {
        let l_under_btn_pin = pins.gp13.into_mode();
        l_under_btn_pin.set_interrupt_enabled(Interrupt::EdgeLow, true);
        l_under_btn_pin.set_interrupt_enabled(Interrupt::EdgeHigh, true);
        critical_section::with(|cs| BUTTON_PIN_UNDER_L.borrow(cs).replace(Some(l_under_btn_pin)));
    }
    {
        let r_under_btn_pin = pins.gp9.into_mode();
        r_under_btn_pin.set_interrupt_enabled(Interrupt::EdgeLow, true);
        r_under_btn_pin.set_interrupt_enabled(Interrupt::EdgeHigh, true);
        critical_section::with(|cs| BUTTON_PIN_UNDER_R.borrow(cs).replace(Some(r_under_btn_pin)));
    }
    {
        let l_front_btn_pin = pins.gp10.into_mode();
        l_front_btn_pin.set_interrupt_enabled(Interrupt::EdgeLow, true);
        l_front_btn_pin.set_interrupt_enabled(Interrupt::EdgeHigh, true);
        critical_section::with(|cs| BUTTON_PIN_FRONT_L.borrow(cs).replace(Some(l_front_btn_pin)));
    }
    {
        let r_front_btn_pin = pins.gp11.into_mode();
        r_front_btn_pin.set_interrupt_enabled(Interrupt::EdgeLow, true);
        r_front_btn_pin.set_interrupt_enabled(Interrupt::EdgeHigh, true);
        critical_section::with(|cs| BUTTON_PIN_FRONT_R.borrow(cs).replace(Some(r_front_btn_pin)));
    }
    {
        let start_btn_pin = pins.gp12.into_mode();
        start_btn_pin.set_interrupt_enabled(Interrupt::EdgeLow, true);
        start_btn_pin.set_interrupt_enabled(Interrupt::EdgeHigh, true);
        critical_section::with(|cs| BUTTON_PIN_START.borrow(cs).replace(Some(start_btn_pin)));
    }
    {
        let select_btn_pin = pins.gp7.into_mode();
        select_btn_pin.set_interrupt_enabled(Interrupt::EdgeLow, true);
        select_btn_pin.set_interrupt_enabled(Interrupt::EdgeHigh, true);
        critical_section::with(|cs| BUTTON_PIN_SELECT.borrow(cs).replace(Some(select_btn_pin)));
    }

    // Setup adc for joystick x / y
    let mut adc = hal::adc::Adc::new(pac.ADC, &mut pac.RESETS);
    let mut l_joy_x_pin = pins.gp26.into_floating_input();
    let mut l_joy_y_pin = pins.gp27.into_floating_input();
    let mut r_joy_x_pin = pins.gp28.into_floating_input();
    let mut r_joy_y_pin = pins.gp29.into_floating_input();

    let mut controller = Controller::default();

    // Allow interrupts last, in case something is not set up fully and IRQ fires
    unsafe {
        pac::NVIC::unmask(pac::Interrupt::IO_IRQ_BANK0);
    }

    // SETUP COMPLETE

    let mut joy_timer = timer.count_down();
    joy_timer.start(10.millis());

    let mut report = JoystickReport::default();
    let mut last_report = JoystickReport::default();

    let mut led_colour = colors::GREEN;
    let mut next_led_colour = led_colour;

    led.write(brightness(core::iter::once(led_colour), 12))
        .unwrap();

    loop {
        if joy_timer.wait().is_ok() {
            // READ STATE
            controller.joy_l.button =
                critical_section::with(|cs| *BUTTON_THUMB_L.borrow(cs).borrow());
            controller.joy_r.button =
                critical_section::with(|cs| *BUTTON_THUMB_R.borrow(cs).borrow());
            controller.under_l = critical_section::with(|cs| *BUTTON_UNDER_L.borrow(cs).borrow());
            controller.under_r = critical_section::with(|cs| *BUTTON_UNDER_R.borrow(cs).borrow());
            controller.front_l = critical_section::with(|cs| *BUTTON_FRONT_L.borrow(cs).borrow());
            controller.front_r = critical_section::with(|cs| *BUTTON_FRONT_R.borrow(cs).borrow());
            controller.joy_l.x = adc.read(&mut l_joy_x_pin).unwrap();
            controller.joy_l.y = adc.read(&mut l_joy_y_pin).unwrap();
            controller.joy_r.x = adc.read(&mut r_joy_x_pin).unwrap();
            controller.joy_r.y = adc.read(&mut r_joy_y_pin).unwrap();

            controller.hid_report(&mut report);

            if last_report != report {
                match joy_hid.device().write_report(&report) {
                    Err(UsbHidError::WouldBlock) => {
                        next_led_colour = colors::DARK_CYAN;
                    }
                    Err(_e) => {
                        next_led_colour = colors::RED;
                        //core::panic!("Unable to write hid report: {:?}", e)
                    }
                    Ok(_) => {
                        next_led_colour = colors::GREEN;
                    } // Setup under / front button interrupt pins
                }
            }
            last_report = report;
        } else {
            next_led_colour = colors::GREEN;
        }

        if !usb_device.poll(&mut [&mut joy_hid]) {
            next_led_colour = colors::ORANGE;
            led.write(brightness(core::iter::once(next_led_colour), 12))
                .unwrap();
            led_colour = next_led_colour;
        }

        if !usb_device.poll(&mut [&mut joy_hid]) {
            next_led_colour = colors::ORANGE;
        }
        if next_led_colour != led_colour {
            led.write(brightness(core::iter::once(next_led_colour), 12))
                .unwrap();
            led_colour = next_led_colour;
        }
    }
}

#[interrupt]
fn IO_IRQ_BANK0() {
    static mut L_THUMB_BUTTON_PIN: Option<ButtonPinThumbL> = None;
    static mut R_THUMB_BUTTON_PIN: Option<ButtonPinThumbR> = None;

    static mut L_UNDER_BUTTON_PIN: Option<ButtonPinUnderL> = None;
    static mut R_UNDER_BUTTON_PIN: Option<ButtonPinUnderR> = None;

    static mut L_FRONT_BUTTON_PIN: Option<ButtonPinFrontL> = None;
    static mut R_FRONT_BUTTON_PIN: Option<ButtonPinFrontR> = None;

    static mut START_BUTTON_PIN: Option<ButtonPinStart> = None;
    static mut SELECT_BUTTON_PIN: Option<ButtonPinSelect> = None;

    if L_THUMB_BUTTON_PIN.is_none() {
        critical_section::with(|cs| *L_THUMB_BUTTON_PIN = BUTTON_PIN_THUMB_L.borrow(cs).take());
    }
    if let Some(pin) = L_THUMB_BUTTON_PIN {
        if pin.interrupt_status(Interrupt::EdgeLow) {
            critical_section::with(|cs| *BUTTON_THUMB_L.borrow(cs).borrow_mut() = true);
            pin.clear_interrupt(Interrupt::EdgeLow);
        } else if pin.interrupt_status(Interrupt::EdgeHigh) {
            critical_section::with(|cs| *BUTTON_THUMB_L.borrow(cs).borrow_mut() = false);
            pin.clear_interrupt(Interrupt::EdgeHigh);
        }
    }

    if R_THUMB_BUTTON_PIN.is_none() {
        critical_section::with(|cs| *R_THUMB_BUTTON_PIN = BUTTON_PIN_THUMB_R.borrow(cs).take());
    }
    if let Some(pin) = R_THUMB_BUTTON_PIN {
        if pin.interrupt_status(Interrupt::EdgeLow) {
            critical_section::with(|cs| *BUTTON_THUMB_R.borrow(cs).borrow_mut() = true);
            pin.clear_interrupt(Interrupt::EdgeLow);
        } else if pin.interrupt_status(Interrupt::EdgeHigh) {
            critical_section::with(|cs| *BUTTON_THUMB_R.borrow(cs).borrow_mut() = false);
            pin.clear_interrupt(Interrupt::EdgeHigh);
        }
    }

    if L_UNDER_BUTTON_PIN.is_none() {
        critical_section::with(|cs| *L_UNDER_BUTTON_PIN = BUTTON_PIN_UNDER_L.borrow(cs).take());
    }
    if let Some(pin) = L_UNDER_BUTTON_PIN {
        if pin.interrupt_status(Interrupt::EdgeLow) {
            critical_section::with(|cs| *BUTTON_UNDER_L.borrow(cs).borrow_mut() = false);
            pin.clear_interrupt(Interrupt::EdgeLow);
        } else if pin.interrupt_status(Interrupt::EdgeHigh) {
            critical_section::with(|cs| *BUTTON_UNDER_L.borrow(cs).borrow_mut() = true);
            pin.clear_interrupt(Interrupt::EdgeHigh);
        }
    }

    if R_UNDER_BUTTON_PIN.is_none() {
        critical_section::with(|cs| *R_UNDER_BUTTON_PIN = BUTTON_PIN_UNDER_R.borrow(cs).take());
    }
    if let Some(pin) = R_UNDER_BUTTON_PIN {
        if pin.interrupt_status(Interrupt::EdgeLow) {
            critical_section::with(|cs| *BUTTON_UNDER_R.borrow(cs).borrow_mut() = false);
            pin.clear_interrupt(Interrupt::EdgeLow);
        } else if pin.interrupt_status(Interrupt::EdgeHigh) {
            critical_section::with(|cs| *BUTTON_UNDER_R.borrow(cs).borrow_mut() = true);
            pin.clear_interrupt(Interrupt::EdgeHigh);
        }
    }

    if L_FRONT_BUTTON_PIN.is_none() {
        critical_section::with(|cs| *L_FRONT_BUTTON_PIN = BUTTON_PIN_FRONT_L.borrow(cs).take());
    }
    if let Some(pin) = L_FRONT_BUTTON_PIN {
        if pin.interrupt_status(Interrupt::EdgeLow) {
            critical_section::with(|cs| *BUTTON_FRONT_L.borrow(cs).borrow_mut() = false);
            pin.clear_interrupt(Interrupt::EdgeLow);
        } else if pin.interrupt_status(Interrupt::EdgeHigh) {
            critical_section::with(|cs| *BUTTON_FRONT_L.borrow(cs).borrow_mut() = true);
            pin.clear_interrupt(Interrupt::EdgeHigh);
        }
    }

    if R_FRONT_BUTTON_PIN.is_none() {
        critical_section::with(|cs| *R_FRONT_BUTTON_PIN = BUTTON_PIN_FRONT_R.borrow(cs).take());
    }
    if let Some(pin) = R_FRONT_BUTTON_PIN {
        if pin.interrupt_status(Interrupt::EdgeLow) {
            critical_section::with(|cs| *BUTTON_FRONT_R.borrow(cs).borrow_mut() = false);
            pin.clear_interrupt(Interrupt::EdgeLow);
        } else if pin.interrupt_status(Interrupt::EdgeHigh) {
            critical_section::with(|cs| *BUTTON_FRONT_R.borrow(cs).borrow_mut() = true);
            pin.clear_interrupt(Interrupt::EdgeHigh);
        }
    }
    if START_BUTTON_PIN.is_none() {
        critical_section::with(|cs| *START_BUTTON_PIN = BUTTON_PIN_START.borrow(cs).take());
    }
    if let Some(pin) = START_BUTTON_PIN {
        if pin.interrupt_status(Interrupt::EdgeLow) {
            critical_section::with(|cs| *BUTTON_START.borrow(cs).borrow_mut() = false);
            pin.clear_interrupt(Interrupt::EdgeLow);
        } else if pin.interrupt_status(Interrupt::EdgeHigh) {
            critical_section::with(|cs| *BUTTON_START.borrow(cs).borrow_mut() = true);
            pin.clear_interrupt(Interrupt::EdgeHigh);
        }
    }

    if SELECT_BUTTON_PIN.is_none() {
        critical_section::with(|cs| *SELECT_BUTTON_PIN = BUTTON_PIN_SELECT.borrow(cs).take());
    }
    if let Some(pin) = SELECT_BUTTON_PIN {
        if pin.interrupt_status(Interrupt::EdgeLow) {
            critical_section::with(|cs| *BUTTON_SELECT.borrow(cs).borrow_mut() = false);
            pin.clear_interrupt(Interrupt::EdgeLow);
        } else if pin.interrupt_status(Interrupt::EdgeHigh) {
            critical_section::with(|cs| *BUTTON_SELECT.borrow(cs).borrow_mut() = true);
            pin.clear_interrupt(Interrupt::EdgeHigh);
        }
    }
}
