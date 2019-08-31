# RusPiRo I²C Bus interface crate

Simple and safe access to the I²C bus available on the Raspberry Pi. This implementation will need the GPIO pins 2 and 3 for exclusive use for the I²C bus.

[![Travis-CI Status](https://api.travis-ci.org/RusPiRo/ruspiro-i2c.svg?branch=master)](https://travis-ci.org/RusPiRo/ruspiro-i2c)
[![Latest Version](https://img.shields.io/crates/v/ruspiro-i2c.svg)](https://crates.io/crates/ruspiro-i2c)
[![Documentation](https://docs.rs/ruspiro-i2c/badge.svg)](https://docs.rs/ruspiro-i2c)
[![License](https://img.shields.io/crates/l/ruspiro-i2c.svg)](https://github.com/RusPiRo/ruspiro-i2c#license)

## Dependency

The I²C implementation requires an allocator to be build into the final binary. It's recommended to use the ``ruspiro-allocator`` with this crate, this could be done by simply activate the feature ``with_allocator`` while defining the dependency to this crate.

## Usage
To use the crate just add the following dependency to your ``Cargo.toml`` file:
```
[dependencies]
ruspiro-i2c = { version = "0.2", features = ["with_allocator"] }
```

Once done the access to the I²C bus interface is available in your rust files like so:
```
use ruspiro_i2c::I2C;

fn demo() {
    I2C.take_for(|i2c| {
        if i2c.initialize(250_000_000, true).is_ok() {
            // now scan I2C devices connected to RPi, this will print their
            // addresses to the console
            i2c.scan();
        }
    });
}
```

To work with a device connected to the I²C bus it first must be retrieved from the I2C interface (it will internally check whether this device is really connected).
Then this device could be used to pass request to it using the I2C API.

```
use ruspiro_i2c::{I2C, I2cDevice};

fn demo() {
    let device = I2C.take_for(|i2c| i2c.get_device(0x68))
                    .expect("no I2C device connected to 0x68");

    I2C.take_for(|i2c| {
        // configure the device...
        // as arbitary example pass value 0x1 to the 8bit register 0x10
        i2c.write_register_u8(device, 0x10, 0x1);
    })
}
```

## License
Licensed under Apache License, Version 2.0, ([LICENSE](LICENSE) or http://www.apache.org/licenses/LICENSE-2.0)