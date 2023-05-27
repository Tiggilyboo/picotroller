# Picotroller

A Raspberry Pi Pico 2040 USB gaming controller.

## Board Support / Requirements

This repository is currently written to use the [Waveshare RF2040 Zero](https://www.waveshare.com/wiki/RP2040-Zero) board. But can be altered to use any RP 2040 board with some modifications to Cargo.toml and associated pinouts.

### Features

1. 2x [Analog Joystick](https://components101.com/modules/joystick-module) with push button
2. 4x Cherry MX keyboard switches used as controller buttons

### Pinout

![Waveshare RP2040 Zero](https://www.waveshare.com/w/upload/2/2b/RP2040-Zero-details-7.jpg)

- GP8 (Pull Up Input Mode) -> Right Joystick Button
- GP14 (Pull Up Input Mode) -> Left Joystick Button
- GP16 (NeoPixel) -> LED
- GP26 (ADC0) -> Left Joystick VRX
- GP27 (ADC1) -> Left Joystick VRY
- GP28 (ADC2) -> Right Joystick VRX
- GP29 (ADC2) -> Right Joystick VRY

## Getting Started

There's a bash script which builds and flashes the firmware to USB, ensure the board is in boot mode by holding the BOOT button as it is powered on.
It's not pretty and uses `udiskctl`, if you don't have this, skip to the next section:

```sh
./build_flash.sh
```

*Manual Way*
Ensure the board is in BOOT mode and is mounted to the filesystem, then run:

```sh
cargo run
```

## Alternatives

1. [GP2040-CE](https://github.com/OpenStickCommunity/GP2040-CE)
