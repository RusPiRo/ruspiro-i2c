/***********************************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: Apache License 2.0
 **********************************************************************************************************************/
#![doc(html_root_url = "https://docs.rs/ruspiro-i2c/0.3.0")]
#![cfg_attr(not(any(test, doctest)), no_std)]

//! # Raspberry Pi I²C bus interface
//!
//! Simple access to the I²C bus available on the Raspberry Pi. When the I²C bus is used this reserves the GPIO pins 2
//! and 3 for exclusive use by the bus.
//!
//! # Usage
//!
//! ```no_run
//! # use ruspiro_i2c::I2C;
//! # fn doc() {
//!     I2C.take_for(|i2c| {
//!         if i2c.initialize(250_000_000, true).is_ok() {
//!             println!("scan I2C devices connected to RPi");
//!             let devices = i2c.scan().unwrap();
//!             for d in devices {
//!                 println!("device detected at 0x{:2X}", d);
//!             }
//!         }
//!     });
//! # }
//! ```
//!
//! To work with a device connected to the I²C bus it's recommended to first check whether this is
//! connected at the specific address. This could be done like so:
//! ```no_run
//! # use ruspiro_i2c::I2C;
//! # fn doc() {
//!     let device_addr = 0x68;
//!     // check if device is connected
//!     I2C.take_for(|i2c| {
//!         if i2c.check_device(device_addr).is_ok() {
//!             // now that we know the device exists and is connected to something with it
//!         }
//!     });
//! # }
//! ```
//! Once done simple use the funtions to write to or read from the device registers as required.
//! 
//! # Features
//!
//! - ``ruspiro_pi3`` is active by default and ensures the proper MMIO base address is used for Raspberry Pi 3
//!

extern crate alloc;
use alloc::vec::Vec;
use ruspiro_register::*;
use ruspiro_singleton::Singleton;

mod interface;

/// Static singleton accessor for the I²C bus peripheral
/// To use the contained i2c API in a safe way use the ``take_for``
/// function passing a clousure that can safely use the resource
/// ```no_run
/// # use ruspiro_i2c::*;
/// # fn doc() {
/// I2C.take_for(|i2c| {
///     // safe access here e.g. to initialize
///     i2c.initialize(250_000_000, true).unwrap();
/// });
/// # }
/// ```
pub static I2C: Singleton<I2cImpl> = Singleton::new(I2cImpl::new());

/// I²C peripheral representation
pub struct I2cImpl {
    initialized: bool,
}

pub type I2cResult<T> = Result<T, &'static str>;

impl I2cImpl {
    /// create a new instance of the I2c implementation. This will only be used to
    /// prepare the static singleton I²C accessor.
    pub(crate) const fn new() -> Self {
        I2cImpl { initialized: false }
    }

    /// Initialize the I²C bus for further usage. This will require the GPIO pins 2 and 3 to be available for usage.
    /// If they have been already occupied before this initialization is called an error will be returned.
    pub fn initialize(&mut self, core_speed: u32, fast_mode: bool) -> I2cResult<()> {
        if !self.initialized {
            interface::initialize(core_speed, fast_mode).and_then(|_| {
                self.initialized = true;
                Ok(())
            })
        } else {
            Ok(())
        }
    }

    /// Scan for I²C devices currently connected to the I²C bus.
    /// The scan will just try to get an acknowledge message from any slave address between
    /// 0x00 and 0x7F. If a device is connected this call succeeds/get's acknowledged and the
    /// corresponding address is put into the returned vector.
    /// # Example
    /// ```no_run
    /// # use ruspiro_i2c::*;
    /// # fn doc() {
    ///     let devices = I2C.take_for(|i2c| i2c.scan()).unwrap();
    ///     for d in devices {
    ///         println!("Device at address: 0x{:X}", d);
    ///     }
    /// # }
    pub fn scan(&self) -> I2cResult<Vec<u8>> {
        self.is_initializied()?;
        Ok(interface::scan_devices())
    }

    /// Checks if a device with the given address is connected to the I²C bus.
    /// # Example
    /// ```no_run
    /// # use ruspiro_i2c::*;
    /// # fn doc() {
    ///     if I2C.take_for(|i2c| i2c.check_device(0x68)).is_ok() {
    ///         println!("device at 0x68 connected");
    ///     }
    /// # }
    /// ```
    pub fn check_device(&self, addr: u8) -> I2cResult<()> {
        self.is_initializied()?;
        interface::check_device(addr)
    }

    /// Read a u8 from a device register
    /// # Example
    /// ```no_run
    /// # use ruspiro_i2c::*;
    /// # fn doc() {
    ///     let value = I2C.take_for(|i2c| i2c.read_register_u8(0x68, 0x20)).unwrap();
    /// # }
    /// ```
    pub fn read_register_u8(&self, device_addr: u8, reg: u8) -> I2cResult<u8> {
        self.is_initializied()?;
        interface::read_reg_u8(device_addr, reg)
    }

    /// Read a u16 from a device register.
    /// As usually all I²C register are 8 Bit wide this will only return a valid value
    /// if the device supports auto-increment of the actual register while reading
    /// # Example
    /// ```no_run
    /// # use ruspiro_i2c::*;
    /// # fn doc() {
    ///     // read_register_u16 will actually read the registers 0x20 and 0x21 and combine
    ///     // both u8 values into the u16 return value.
    ///     let value = I2C.take_for(|i2c| i2c.read_register_u16(0x68, 0x20)).unwrap();
    /// # }
    /// ```
    pub fn read_register_u16(&self, device_addr: u8, reg: u8) -> I2cResult<u16> {
        self.is_initializied()?;
        interface::read_reg_u16(device_addr, reg)
    }

    /// Read a u8 array from a device register.
    /// As usually all I²C register are 8 Bit wide this will only return a valid value
    /// if the device supports auto-increment of the actual register while reading
    /// # Example
    /// ```no_run
    /// # use ruspiro_i2c::*;
    /// # fn doc() {
    ///     let mut buffer: [u8; 4] = [0; 4];
    ///     // the buffer read will actuall read the registers 0x20, 0x21, 0x22, 0x23
    ///     // and put the data into the byte buffer given (if register auto increment is supported
    ///     // by this device)
    ///     let _ = I2C.take_for(|i2c| i2c.read_register_buff(0x68, 0x20, &mut buffer)).unwrap();
    /// # }
    /// ```
    pub fn read_register_buff(
        &self,
        device_addr: u8,
        reg: u8,
        buffer: &mut [u8],
    ) -> I2cResult<usize> {
        self.is_initializied()?;
        interface::read_reg_data(device_addr, reg, buffer)
    }

    /// Read a specific field from a 8 Bit device register.
    /// # Example
    /// ```no_run
    /// # use ruspiro_i2c::*;
    /// # use ruspiro_register::*;
    /// # fn doc() {
    ///     // define an arbitrary register field with 1 bit size at offset 2
    ///     let field = RegisterField::<u8>::new(1, 2);
    ///     let field_value = I2C.take_for(|i2c| i2c.read_register_field(0x68, 0x20, field)).unwrap();
    /// # }
    /// ```
    pub fn read_register_field(
        &self,
        device_addr: u8,
        reg: u8,
        field: RegisterField<u8>,
    ) -> I2cResult<RegisterFieldValue<u8>> {
        self.is_initializied()?;
        let value = interface::read_reg_u8(device_addr, reg)?;
        Ok(RegisterFieldValue::<u8>::new(field, value >> field.shift()))
    }

    /// Write u8 data to a device without specifying a register.
    /// This is helpful for devices that may not provide any registers or have only one register
    /// to wrte to. In those cases the device accepts the data without specifying the register in
    /// the first place.
    /// # Example
    /// ```no_run
    /// # use ruspiro_i2c::*;
    /// # fn doc() {
    ///     I2C.take_for(|i2c| i2c.write_u8(0x68, 12)).unwrap();
    /// # }
    /// ```
    pub fn write_u8(&self, device_addr: u8, data: u8) -> I2cResult<()> {
        self.is_initializied()?;
        interface::write_raw_u8(device_addr, data)
    }

    /// Write u8 data to a device register
    /// # Example
    /// ```no_run
    /// # use ruspiro_i2c::*;
    /// # fn doc() {
    ///     I2C.take_for(|i2c| i2c.write_register_u8(0x68, 0x20, 12)).unwrap();
    /// # }
    /// ```
    pub fn write_register_u8(&self, device_addr: u8, reg: u8, data: u8) -> I2cResult<()> {
        self.is_initializied()?;
        interface::write_reg_u8(device_addr, reg, data)
    }

    /// Write u16 data to a device register.
    /// As usually all I²C register are 8 Bit wide this will only properly write the value
    /// if the device supports auto-increment of the actual register while reading
    /// # Example
    /// ```no_run
    /// # use ruspiro_i2c::*;
    /// # fn doc() {
    ///     // this will actually write 0x12 to register 0x20 and 0xab to register 0x21
    ///     // if the device supports auto increment of registers for writes
    ///     I2C.take_for(|i2c| i2c.write_register_u16(0x68, 0x20, 0x12ab)).unwrap();
    /// # }
    /// ```
    pub fn write_register_u16(&self, device_addr: u8, reg: u8, data: u16) -> I2cResult<()> {
        self.is_initializied()?;
        interface::write_reg_u16(device_addr, reg, data)
    }

    /// Write a u8 array to a device register.
    /// As usually all I²C register are 8 Bit wide this will only properly write the value
    /// if the device supports auto-increment of the actual register while reading
    /// # Example
    /// ```no_run
    /// # use ruspiro_i2c::*;
    /// # fn doc() {
    ///     let data: [u8; 3] = [0, 1, 2];
    ///     I2C.take_for(|i2c| i2c.write_register_buff(0x68, 0x20, &data)).unwrap();
    /// # }
    /// ```
    pub fn write_register_buff(&self, device_addr: u8, reg: u8, data: &[u8]) -> I2cResult<()> {
        self.is_initializied()?;
        interface::write_reg_data(device_addr, reg, data)
    }

    /// Write a specific register field to a 8 Bit device register.
    /// # Example
    /// ```no_run
    /// # use ruspiro_i2c::*;
    /// # use ruspiro_register::*;
    /// # fn doc() {
    ///     // define an arbitrary field with bit size 2 and offset 3
    ///     let field = RegisterField::<u8>::new(2, 3);
    ///     // define the field value
    ///     let field_value = RegisterFieldValue::<u8>::new(field, 0b10);
    ///     let value = I2C.take_for(|i2c| i2c.write_register_field(0x68, 0x20, field_value)).unwrap();
    /// # }
    /// ```
    pub fn write_register_field(
        &self,
        device_addr: u8,
        reg: u8,
        value: RegisterFieldValue<u8>,
    ) -> I2cResult<()> {
        self.is_initializied()?;
        let old_value = self.read_register_u8(device_addr, reg)?;
        let new_value = (old_value & !value.mask()) | value.raw_value();
        interface::write_reg_u8(device_addr, reg, new_value)
    }

    #[inline(always)]
    fn is_initializied(&self) -> I2cResult<()> {
        if !self.initialized {
            Err("I2C Bus not initialized")
        } else {
            Ok(())
        }
    }
}
