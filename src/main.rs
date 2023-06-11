#![no_std]
#![no_main]

use defmt_rtt as _;

use core::cell::RefCell;

use cortex_m::prelude::{_embedded_hal_adc_OneShot, _embedded_hal_timer_CountDown};
use critical_section::Mutex;
use panic_halt as _;
use smart_leds::colors;
use usb_device::class_prelude::UsbBusAllocator;
use usb_device::prelude::{UsbDeviceBuilder, UsbVidPid};
use usbd_human_interface_device::UsbHidError;
use usbd_human_interface_device::usb_class::UsbHidClassBuilder;
use fugit::ExtU32;
use smart_leds::{
    brightness, 
    SmartLedsWrite,
};
use ws2812_pio::Ws2812;
use waveshare_rp2040_zero as bsp;
use bsp::{
    entry,
    Pins, 
};
use bsp::hal as hal;
use hal::{
    clocks::init_clocks_and_plls,
    clocks::Clock,
    pac,
    pac::interrupt,
    gpio::Interrupt,
    watchdog::Watchdog,
    Sio,
    pio::PIOExt,
    timer::Timer,
    gpio,
};

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
type ButtonPinTriggerL = gpio::Pin<gpio::bank0::Gpio13, gpio::PullDownInput>;
type ButtonPinTriggerR = gpio::Pin<gpio::bank0::Gpio9, gpio::PullDownInput>;

static BUTTON_PIN_THUMB_L: Mutex<RefCell<Option<ButtonPinThumbL>>> = Mutex::new(RefCell::new(None));
static BUTTON_PIN_THUMB_R: Mutex<RefCell<Option<ButtonPinThumbR>>> = Mutex::new(RefCell::new(None));
static BUTTON_PIN_TRIGGER_L: Mutex<RefCell<Option<ButtonPinTriggerL>>> = Mutex::new(RefCell::new(None));
static BUTTON_PIN_TRIGGER_R: Mutex<RefCell<Option<ButtonPinTriggerR>>> = Mutex::new(RefCell::new(None));

// state
static BUTTON_THUMB_L: Mutex<RefCell<bool>> = Mutex::new(RefCell::new(false));
static BUTTON_THUMB_R: Mutex<RefCell<bool>> = Mutex::new(RefCell::new(false));
static BUTTON_TRIGGER_L: Mutex<RefCell<bool>> = Mutex::new(RefCell::new(false));
static BUTTON_TRIGGER_R: Mutex<RefCell<bool>> = Mutex::new(RefCell::new(false));

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
    led.write(brightness(core::iter::once(colors::RED), 8)).unwrap();

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
    
    led.write(brightness(core::iter::once(colors::RED), 6)).unwrap();

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
    {
        let l_trig_btn_pin = pins.gp13.into_mode();
        l_trig_btn_pin.set_interrupt_enabled(Interrupt::EdgeLow, true);
        l_trig_btn_pin.set_interrupt_enabled(Interrupt::EdgeHigh, true);
        critical_section::with(|cs| BUTTON_PIN_TRIGGER_L.borrow(cs).replace(Some(l_trig_btn_pin)));
    }
    {
        let r_trig_btn_pin = pins.gp9.into_mode();
        r_trig_btn_pin.set_interrupt_enabled(Interrupt::EdgeLow, true);
        r_trig_btn_pin.set_interrupt_enabled(Interrupt::EdgeHigh, true);
        critical_section::with(|cs| BUTTON_PIN_TRIGGER_R.borrow(cs).replace(Some(r_trig_btn_pin)));
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

    led.write(brightness(core::iter::once(led_colour), 12)).unwrap();

    loop {
        if joy_timer.wait().is_ok() {
            // READ STATE
            controller.joy_l.button = critical_section::with(|cs| *BUTTON_THUMB_L.borrow(cs).borrow());
            controller.joy_r.button = critical_section::with(|cs| *BUTTON_THUMB_R.borrow(cs).borrow());
            controller.trigger_l = critical_section::with(|cs| *BUTTON_TRIGGER_L.borrow(cs).borrow());
            controller.trigger_r = critical_section::with(|cs| *BUTTON_TRIGGER_R.borrow(cs).borrow());
            controller.joy_l.x = adc.read(&mut l_joy_x_pin).unwrap();
            controller.joy_l.y = adc.read(&mut l_joy_y_pin).unwrap();
            controller.joy_r.x = adc.read(&mut r_joy_x_pin).unwrap();
            controller.joy_r.y = adc.read(&mut r_joy_y_pin).unwrap();

            controller.hid_report(&mut report);

            if last_report != report {
                match joy_hid.device().write_report(&report) {
                    Err(UsbHidError::WouldBlock) => {
                        next_led_colour = colors::DARK_CYAN;
                    },
                    Err(_e) => {
                        next_led_colour = colors::RED;
                        //core::panic!("Unable to write hid report: {:?}", e)
                    },
                    Ok(_) => {
                        next_led_colour = colors::GREEN;
                    }
                }
            }
            last_report = report;
        } else {
            next_led_colour = colors::GREEN;
        }

        if !usb_device.poll(&mut [&mut joy_hid]) {
            next_led_colour = colors::ORANGE;
        }
        if next_led_colour != led_colour {
            led.write(brightness(core::iter::once(next_led_colour), 12)).unwrap();
            led_colour = next_led_colour;
        }
    }
}

#[interrupt]
fn IO_IRQ_BANK0() {
    static mut L_THUMB_BUTTON_PIN: Option<ButtonPinThumbL> = None;
    static mut R_THUMB_BUTTON_PIN: Option<ButtonPinThumbR> = None;

    static mut L_TRIGGER_BUTTON_PIN: Option<ButtonPinTriggerL> = None;
    static mut R_TRIGGER_BUTTON_PIN: Option<ButtonPinTriggerR> = None;

    if L_THUMB_BUTTON_PIN.is_none() {
        critical_section::with(|cs| *L_THUMB_BUTTON_PIN = BUTTON_PIN_THUMB_L.borrow(cs).take());
    }
    if let Some(pin) = L_THUMB_BUTTON_PIN {
        if pin.interrupt_status(Interrupt::EdgeLow) {
            critical_section::with(|cs| *BUTTON_THUMB_L.borrow(cs).borrow_mut() = true);
            pin.clear_interrupt(Interrupt::EdgeLow);
        }
        else if pin.interrupt_status(Interrupt::EdgeHigh) {
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
        }
        else if pin.interrupt_status(Interrupt::EdgeHigh) {
            critical_section::with(|cs| *BUTTON_THUMB_R.borrow(cs).borrow_mut() = false);
            pin.clear_interrupt(Interrupt::EdgeHigh);
        }
    }
    
    if L_TRIGGER_BUTTON_PIN.is_none() {
        critical_section::with(|cs| *L_TRIGGER_BUTTON_PIN = BUTTON_PIN_TRIGGER_L.borrow(cs).take());
    }
    if let Some(pin) = L_TRIGGER_BUTTON_PIN {
        if pin.interrupt_status(Interrupt::EdgeLow) {
            critical_section::with(|cs| *BUTTON_TRIGGER_L.borrow(cs).borrow_mut() = false);
            pin.clear_interrupt(Interrupt::EdgeLow);
        }
        else if pin.interrupt_status(Interrupt::EdgeHigh) {
            critical_section::with(|cs| *BUTTON_TRIGGER_L.borrow(cs).borrow_mut() = true);
            pin.clear_interrupt(Interrupt::EdgeHigh);
        }
    }
    
    if R_TRIGGER_BUTTON_PIN.is_none() {
        critical_section::with(|cs| *R_TRIGGER_BUTTON_PIN = BUTTON_PIN_TRIGGER_R.borrow(cs).take());
    }
    if let Some(pin) = R_TRIGGER_BUTTON_PIN {
        if pin.interrupt_status(Interrupt::EdgeLow) {
            critical_section::with(|cs| *BUTTON_TRIGGER_R.borrow(cs).borrow_mut() = false);
            pin.clear_interrupt(Interrupt::EdgeLow);
        }
        else if pin.interrupt_status(Interrupt::EdgeHigh) {
            critical_section::with(|cs| *BUTTON_TRIGGER_R.borrow(cs).borrow_mut() = true);
            pin.clear_interrupt(Interrupt::EdgeHigh);
        }
    }
}

