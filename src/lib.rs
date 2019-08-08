/*********************************************************************************************************************** 
 * Copyright (c) 2019 by the authors
 * 
 * Author: André Borrmann 
 * License: Apache License 2.0
 **********************************************************************************************************************/
#![doc(html_root_url = "https://docs.rs/ruspiro-i2c/0.1.0")]
#![no_std]

//! # I²C bus interface
//! 
//! 
//! 
extern crate alloc;
use alloc::vec::Vec;
use ruspiro_singleton::Singleton;
use ruspiro_console::*;

mod interface;

/// static singleton accessor for the I²C bus peripheral
pub static I2C: Singleton<I2cImpl> = Singleton::new(I2cImpl::new());

/// I²C peripheral representation
pub struct I2cImpl {
    initialized: bool,
}

/// I²C device representation
pub struct I2cDevice {
    addr: u32,
}

type I2cResult<T> = Result<T, &'static str>;

impl I2cImpl {
    /// create a new instance of the I2c implementation. This will only be used to
    /// prepare the static singleton I²C accessor.
    pub(crate) const fn new() -> Self {
        I2cImpl {
            initialized: false,
        }
    }

    /// Initialize the I²C bus for further usage. This will require the GPIO pins 2 and 3 to be available for usage.
    /// If they have been already occupied before this initialization is called an error will be returned.
    pub fn initialize(&mut self, core_speed: u32, fast_mode: bool) -> I2cResult<()>{
        if !self.initialized {
            interface::initialize(core_speed, fast_mode)
                .and_then(|_| { 
                    self.initialized = true;
                    Ok(())
                })
        } else {
            Ok(())
        }
    }

    /// Scan for I²C devices currently connected to the I²C bus. The scan will just try to get an acknowledge message
    /// from any slave address between 0x00 and 0x7F. If a device is connected this call succeeds and the corresponding
    /// address is written to the console. This function is typically only used to check for device addresses when a new
    /// device is connected but it's address is not properly documented.
    pub fn scan(&self) {
        let devices = interface::scan_devices();
        for dev in devices {
            info!("device detected at {:2X}", dev);
        };
    }

    /// Get a new I²C device with the given address. This functions checks whether ther is really a device connected
    /// with the given address to the I²C bus.
    /// TODO: ensure there will not be more then 1 request for a device on the same address
    pub fn get_device(&self, addr: u8) -> I2cResult<I2cDevice> {
        interface::check_device(addr)
            .map(|_| {
                I2cDevice::new(addr as u32)
            })
    }

    /// Read a u8 from a device register
    pub fn read_register_u8(&self, device: I2cDevice, reg: u8) -> I2cResult<u8> {
        interface::read_reg_u8(device.addr, reg as u32)
    }

    /// Read a u16 from a device register
    pub fn read_register_u16(&self, device: I2cDevice, reg: u8) -> I2cResult<u16> {
        interface::read_reg_u16(device.addr, reg as u32)
    }

    /// Read a u8 array from a device register
    pub fn read_register_buff(&self, device: I2cDevice, reg: u8, len: u16) -> I2cResult<Vec<u8>> {
        interface::read_reg_data(device.addr, reg as u32, len)
    }

    /// Read a specific bit from a device register. Returns Ok(true) if the bit is set
    /// or Ok(false) if not.
    pub fn read_register_bit(&self, device: I2cDevice, reg: u8, offset: u16) -> I2cResult<bool> {
        interface::read_reg_bits(device.addr, reg as u32, offset, 1)
            .map(|v| if v == 1 { true } else { false })
    }

    /// Read specific bits from a device register. The bits in the result are shifted to the right
    /// so they appear as 0 offset.
    pub fn read_register_bits(&self, device: I2cDevice, reg: u8, offset: u16, bits: u16) -> I2cResult<u8> {
        interface::read_reg_bits(device.addr, reg as u32, offset, bits)
    }

    /// Write u8 data to a device without specifying a register
    pub fn write_u8(&self, device: I2cDevice, data: u8) -> I2cResult<()> {
        interface::write_raw_u8(device.addr, data)
    }

    /// Write u8 data to a device register
    pub fn write_register_u8(&self, device: I2cDevice, reg: u8, data: u8) -> I2cResult<()> {
        interface::write_reg_u8(device.addr, reg as u32, data)
    }

    /// Write u16 data to a device register
    pub fn write_register_u16(&self, device: I2cDevice, reg: u8, data: u16) -> I2cResult<()> {
        interface::write_reg_u16(device.addr, reg as u32, data)
    }

    /// Write a u8 array to a device register
    pub fn write_register_buff(&self, device: I2cDevice, reg: u8, data: &[u8]) -> I2cResult<()> {
        interface::write_reg_data(device.addr, reg as u32, data)
    }

    /// Write a specific bit to a device register. If set is true the bit will be set to 1
    pub fn write_register_bit(&self, device: I2cDevice, reg: u8, offset: u16, set: bool) -> I2cResult<()> {
        interface::write_reg_bits(device.addr, reg as u32, offset, 1, set as u8)
    }

    /// Read specific bits from a device register. The bits in the result are shifted to the right
    /// so they appear as 0 offset.
    pub fn write_register_bits(&self, device: I2cDevice, reg: u8, offset: u16, bits: u16, value: u8) -> I2cResult<()> {
        interface::write_reg_bits(device.addr, reg as u32, offset, bits, value)
    }
}

impl I2cDevice {
    /// create a new I²C deivce instance. This instantiation is only available inside this crate and will be created
    /// from the I²C implementation only to verify there is really a device connected to this address.
    pub(crate) fn new(addr: u32) -> Self {
        I2cDevice {
            addr: addr,
        }
    }
}