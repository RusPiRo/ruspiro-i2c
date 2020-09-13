# RusPiRo I²C Bus interface crate

Simple and safe access to the I²C bus available on the Raspberry Pi. This implementation will need
the GPIO pins 2 and 3 for exclusive use for the I²C bus.

[![Travis-CI Status](https://api.travis-ci.org/RusPiRo/ruspiro-i2c.svg?branch=master)](https://travis-ci.org/RusPiRo/ruspiro-i2c)
[![Latest Version](https://img.shields.io/crates/v/ruspiro-i2c.svg)](https://crates.io/crates/ruspiro-i2c)
[![Documentation](https://docs.rs/ruspiro-i2c/badge.svg)](https://docs.rs/ruspiro-i2c)
[![License](https://img.shields.io/crates/l/ruspiro-i2c.svg)](https://github.com/RusPiRo/ruspiro-i2c#license)

## Dependency

The I²C implementation requires an allocator to be build into the final binary. It's recommended to
use the ``ruspiro-allocator`` with this crate.

## Usage

To use the crate just add the following dependency to your ``Cargo.toml`` file:

```toml
[dependencies]
ruspiro-i2c = "0.3"
```

Once done the access to the I²C bus interface is available in your rust files like so:

```rust
use ruspiro_i2c::I2C;

fn demo() {
    I2C.take_for(|i2c| {
        if i2c.initialize(250_000_000, true).is_ok() {
            // now scan I2C devices connected to RPi, this will print their
            // addresses to the console
            let devices = i2c.scan().unwrap();
            for d in devices {
                info!("device detected at 0x{:2X}", d);
            }
        }
    });
}
```

To configure and use a device connected to the I²C bus you can simply use the provided functions
provided when taking the I2C. First off you may check whether the device is connected to the bus
before continuing with the config:

```rust
use ruspiro_i2c::I2C;

fn demo() {
    let device_addr = 0x68;
    I2C.take_for(|i2c| {
        if i2c.check_device(device_addr).is_ok() {
            // configure the device...
            // as arbitary example pass value 0x1 to the 8bit register 0x10
            i2c.write_register_u8(device_addr, 0x10, 0x1);
        }
    });
}
```

## License

Licensed under Apache License, Version 2.0, ([LICENSE](LICENSE) or http://www.apache.org/licenses/LICENSE-2.0)