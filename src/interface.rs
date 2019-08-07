/*********************************************************************************************************************** 
 * Copyright (c) 2019 by the authors
 * 
 * Author: André Borrmann 
 * License: Apache License 2.0
 **********************************************************************************************************************/

//! # I²C internal interface
//! 
//! Internal hardware related implementation
//! 
extern crate alloc;
use alloc::{vec, vec::Vec};

use ruspiro_register::define_registers;
use ruspiro_timer as timer;
use ruspiro_gpio::GPIO;
use ruspiro_console::*;

use crate::I2C;

#[cfg(feature="ruspiro_pi3")]
const PERIPHERAL_BASE: u32 = 0x3F00_0000;

const I2C_BASE: u32 = PERIPHERAL_BASE + 0x0080_4000;
const I2C_MAX_BYTES: u16 = 16;
const I2C_DEFAULT_WAIT: u32 = 2000;

type I2cResult<T> = Result<T, &'static str>;

pub(crate) fn initialize(core_speed: u32, fast_mode: bool) -> I2cResult<()> {
    // when I2C is about to be initialized reserve GPIO Pins 2 and 3
    // as the I2C bus pins
    GPIO.take_for(|gpio| gpio.get_pin(2))
        .map(|pin| pin.to_alt_f0())
        .and_then(|_| GPIO.take_for(|gpio| gpio.get_pin(3)))
        .map(|pin| pin.to_alt_f0())
        .and_then(|_| {
            // both pin's configured
            // setup the I2C speed
            let clock_divisor = if fast_mode {
                core_speed / 400_000
            } else {
                core_speed / 100_000
            };

            I2C_REG_CDIV::Register.set(clock_divisor);            
            Ok(())
        })
}

/// Scan for I2C devices currently connected to the I2C bus. The scan will just try to get an acknowledge message
/// from any slave address between 0x00 and 0x7F. If a device is connected this call succeeds and the corresponding
/// address is written to the console
pub(crate) fn scan_devices() -> Vec<u8> {
    let mut r: Vec<u8> = vec![];
    for addr in 0x00..0x80 {
        I2C_REG_A::Register.set(addr);
        I2C_REG_DLEN::Register.set(1);
        I2C_REG_S::Register.set(StatusFlag::Clear as u32);
        I2C_REG_C::Register.set(ControlFlag::StartRead as u32);

        if wait_i2c_done(100).is_ok() {
            r.push(addr as u8);
        };
    };

    r
}

pub(crate) fn check_device(addr: u8) -> I2cResult<()> {
    I2C_REG_A::Register.set(addr as u32);
    I2C_REG_DLEN::Register.set(1);
    I2C_REG_S::Register.set(StatusFlag::Clear as u32);
    I2C_REG_C::Register.set(ControlFlag::StartRead as u32);

    wait_i2c_done(100)
}

pub(crate) fn read_reg_u8(addr: u32, reg: u32) -> I2cResult<u8> {
    // aquire atmic access to the I2C peripheral, this is necessary as the device
    // is accessible outside the I2C peripheral.
    I2C.take_for(|_| {
        write_register(addr, reg)
            .and_then(|_| {
                I2C_REG_DLEN::Register.set(1);
                I2C_REG_S::Register.set(StatusFlag::Clear as u32);
                I2C_REG_C::Register.set(ControlFlag::StartRead as u32);
                wait_i2c_done(I2C_DEFAULT_WAIT)
                    .map_err(|e| {
                        error!("read u8 addr:{:x} / reg:{:x} - {} - status {:x}", addr, reg, e, I2C_REG_S::Register.get());
                        e
                    })
            }).and_then(|_| {
                Ok(read_fifo(1)[0])
            })
    })
}

pub(crate) fn read_reg_u16(addr: u32, reg: u32) -> I2cResult<u16> {
    read_reg_data(addr, reg, 2)
        .and_then(|data| {
            Ok((data[0] as u16) << 8 | (data[1] as u16))
        })
}

pub(crate) fn read_reg_bits(addr: u32, reg: u32, offset: u16, bits: u16) -> I2cResult<u8> {
    read_reg_u8(addr, reg)
        .and_then(|data| {
            let mask = ((1 << bits) - 1) << offset;
            Ok((data & mask) >> offset)
        })
}

pub(crate) fn read_reg_data(addr: u32, reg: u32, count: u16) -> I2cResult<Vec<u8>> {
    // aquire atmic access to the I2C peripheral, this is necessary as the device
    // is accessible outside the I2C peripheral.
    I2C.take_for(|_| {
        write_register(addr, reg)
            .and_then(|_| {                    
                I2C_REG_DLEN::Register.set(count as u32);
                I2C_REG_S::Register.set(StatusFlag::Clear as u32);
                I2C_REG_C::Register.set(ControlFlag::StartRead as u32);
                wait_i2c_done(I2C_DEFAULT_WAIT)
                    .map_err(|e| {
                        error!("read data from addr:{:x} / reg:{:x} - {}", addr, reg, e);
                        e
                    })
            }).and_then(|_| {
                let mut data: Vec<u8> = Vec::with_capacity(count as usize);
                let chunks = count / I2C_MAX_BYTES;
                let mut remainder = count;
                for _ in 0..chunks+1 {
                    let size = if remainder > I2C_MAX_BYTES { I2C_MAX_BYTES } else { remainder };
                    data.extend(read_fifo(size as u8).into_iter());
                    remainder -= I2C_MAX_BYTES;
                }
                Ok(data)
            })
    })
}

pub(crate) fn write_raw_u8(addr: u32, data: u8) -> I2cResult<()> {
    // aquire atmic access to the I2C peripheral, this is necessary as the device
    // is accessible outside the I2C peripheral.
    I2C.take_for(|_| {
        // clear status flags
        I2C_REG_S::Register.set(StatusFlag::Clear as u32);
        // clear FiFo data in case FiFo data has remained from previous calls
        I2C_REG_C::Register.set(ControlFlag::Clear as u32);
        // set the slave address we would like to send data to and the register id
        I2C_REG_A::Register.set(addr);
        I2C_REG_DLEN::Register.set(1);
        I2C_REG_FIFO::Register.set(data as u32);
        // transmit the data
        I2C_REG_C::Register.set(ControlFlag::StartWrite as u32);

        wait_i2c_done(I2C_DEFAULT_WAIT)
            .map_err(|e| {
                error!(" write raw to addr:{:x} - {}", addr, e);
                e
            })
    })
}

pub(crate) fn write_reg_u8(addr: u32, reg: u32, data: u8) -> I2cResult<()> {
    // aquire atmic access to the I2C peripheral, this is necessary as the device
    // is accessible outside the I2C peripheral.
    I2C.take_for(|_| {
        // clear status flags
        I2C_REG_S::Register.set(StatusFlag::Clear as u32);
        // clear FiFo data in case FiFo data has remained from previous calls
        I2C_REG_C::Register.set(ControlFlag::Clear as u32);
        // set the slave address we would like to send data to and the register id
        I2C_REG_A::Register.set(addr);
        I2C_REG_DLEN::Register.set(2);
        I2C_REG_FIFO::Register.set(reg);
        I2C_REG_FIFO::Register.set(data as u32);
        // transmit the data
        I2C_REG_C::Register.set(ControlFlag::StartWrite as u32);

        wait_i2c_done(I2C_DEFAULT_WAIT)
            .map_err(|e| {
                error!("write u8 to addr:{:x} / reg:{:x} - {}", addr, reg, e);
                e
            })
    })
}

pub(crate) fn write_reg_u16(addr: u32, reg: u32, data: u16) -> I2cResult<()> {
    let buffer: [u8;2] = [(data >> 8) as u8, (data & 0xFF) as u8];
    write_reg_data(addr, reg, &buffer)
}

pub(crate) fn write_reg_bits(addr: u32, reg: u32, offset: u16, bits: u16, value: u8) -> I2cResult<()> {
    read_reg_u8(addr, reg)
        .and_then(|current| {
            let mask = ((1 << bits) - 1) << offset;
            let data = (current & !mask) | value << offset;
            write_reg_u8(addr, reg, data)
        })
}

pub(crate) fn write_reg_data(addr: u32, reg: u32, data: &[u8]) -> I2cResult<()> {
    // aquire atmic access to the I2C peripheral, this is necessary as the device
    // is accessible outside the I2C peripheral.
    I2C.take_for(|_| {
        let mut data_len = data.len() as u32;
        // clear status flags
        I2C_REG_S::Register.set(StatusFlag::Clear as u32);
        // clear FiFo data in case FiFo data has remained from previous calls
        I2C_REG_C::Register.set(ControlFlag::Clear as u32);
        // set the slave address we would like to send data to and the register id
        I2C_REG_A::Register.set(addr);
        I2C_REG_DLEN::Register.set(data_len+1);
        I2C_REG_FIFO::Register.set(reg);
        // transmit the data
        I2C_REG_C::Register.set(ControlFlag::StartWrite as u32);
        let chunks = data_len/I2C_MAX_BYTES as u32;
        for chunk in 0..chunks+1 {
            let idx: usize = (chunk*data_len) as usize;
            let len: usize = if data_len > I2C_MAX_BYTES as u32 {
                I2C_MAX_BYTES as usize
            } else {
                data_len as usize
            };
            write_fifo(&data[idx..len]);
            data_len -= I2C_MAX_BYTES as u32;
        }

        wait_i2c_done(I2C_DEFAULT_WAIT)
            .map_err(|e| {
                error!("write data to addr:{:x} / reg:{:x} - {}", addr, reg, e);
                e
            })
    })
}


/// Wait until the current I2C operation has been finished/acknowledged
/// Returns an [Err] in case of a timeout or not beein acknowledged
fn wait_i2c_done(tries: u32) -> I2cResult<()>{
    for _ in 0..tries {
        if I2C_REG_S::Register.read(I2C_REG_S::TRANS_DONE) != 0 {
            if I2C_REG_S::Register.read(I2C_REG_S::ACK_ERROR) == 0 {
                return Ok(());
            } else {
                return Err("I2C transmit not acknowledged");
            }
        }
        timer::sleepcycles(1000);
    }
    Err("time out waiting for I2C transmit")
}

/// Write the register to the I2C device we would like to access next (e.g. write to)
fn write_register(addr: u32, reg: u32) -> I2cResult<()> {
    // set the slave address we would like to send data to and the register id
    I2C_REG_A::Register.set(addr);
    I2C_REG_DLEN::Register.set(1);
    I2C_REG_FIFO::Register.set(reg);
    // transmit the data
    I2C_REG_S::Register.set(StatusFlag::Clear as u32);
    I2C_REG_C::Register.set(ControlFlag::StartWrite as u32);

    wait_i2c_done(I2C_DEFAULT_WAIT)
        .map_err(|e| {
            error!("write register addr:{:x} / reg:{:x} - {}", addr, reg, e);
            e
        })
}

/// Read the data from the I2C FIFO register
fn read_fifo(count: u8) -> Vec<u8> {
    let mut data: Vec<u8> = Vec::with_capacity(count as usize);
    let num = if count as u16 > I2C_MAX_BYTES { I2C_MAX_BYTES } else { count as u16};
    for _ in 0..num {
        while I2C_REG_S::Register.read(I2C_REG_S::RX_DATA) == 0 {};
        data.push((I2C_REG_FIFO::Register.get() & 0xFF) as u8);
    }
    data
}

fn write_fifo(data: &[u8]) {
    for i in 0..data.len() {
        while I2C_REG_S::Register.read(I2C_REG_S::TX_DATA) == 0 {};
        I2C_REG_FIFO::Register.set(data[i] as u32);
    }
}

// I2C register definitions
define_registers! [
    I2C_REG_C:    ReadWrite<u32> @ I2C_BASE + 0x00 => [ // control register
        ENABLE     OFFSET(15),
        IRQ_RX     OFFSET(10),
        IRQ_TX     OFFSET(9),
        IRQ_DONE   OFFSET(8),
        STARTTRANS OFFSET(7),
        CLEAR      OFFSET(4),
        READ       OFFSET(1),
        WRITE      OFFSET(0)

    ],
    I2C_REG_S:    ReadWrite<u32> @ I2C_BASE + 0x04 => [ // status register
        CLK_TIMEOUT  OFFSET(9),
        ACK_ERROR    OFFSET(8),
        RX_FULL      OFFSET(7),
        TX_EMPTY     OFFSET(6),
        RX_DATA      OFFSET(5),
        TX_DATA      OFFSET(4),
        RX_NEEDREAD  OFFSET(3),
        TX_NEEDWRITE OFFSET(2),
        TRANS_DONE   OFFSET(1),
        TRANS_ACTIVE OFFSET(0)
    ],
    I2C_REG_DLEN: ReadWrite<u32> @ I2C_BASE + 0x08 => [   // data len register
        DATA OFFSET(0) BITS(16)
    ],
    I2C_REG_A:    ReadWrite<u32> @ I2C_BASE + 0x0C, // slave address register
    I2C_REG_FIFO: ReadWrite<u32> @ I2C_BASE + 0x10,
    I2C_REG_CDIV: ReadWrite<u32> @ I2C_BASE + 0x14,
    I2C_REG_DEL:  ReadWrite<u32> @ I2C_BASE + 0x18,
    I2C_REG_CLKT: ReadWrite<u32> @ I2C_BASE + 0x1C
];

#[repr(u32)]
enum ControlFlag {
    Enable =     1 << 15,
    StartTrans = 1 << 7,
    Clear =      1 << 4,
    Read  =      1 << 1,
    Write =      1 << 0,
    StartRead  = (1 << 15) | (1 << 7) | (1 << 4) | (1 << 0),
    StartWrite = (1 << 15) | (1 << 7) | (0 << 0),
}

#[repr(u32)]
enum StatusFlag {
    ClckTimeout = 	1 << 9,	// 1 Slave has held the SCL signal longer than allowed high
    AckError    =	1 << 8,	// 1 Slave address acknowledge error
    RxFull      =	1 << 7,	// 1 FIFO is full
    TxEmpty     =	1 << 6,	// 1 FIFO is empty
    RxData      = 	1 << 5,	// 1 FIFO contains at least one byte
    TxData      =	1 << 4,	// 1 FIFO can accept data
    RxNeedRead  =	1 << 3,	// 1 FIFO is full and needs reading from the FIFO
    TxNeedWrite =	1 << 2,	// 1 FIFO is less than full and needs writing to the FIFO
    TransDone   =   1 << 1,	// 1 if transfer is complete
    TransActive = 	1 << 0,	// 1 if transfer is active
    Clear       = (1 << 9) | (1 << 8) | (1 << 1),
}