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

use ruspiro_gpio::GPIO;
use ruspiro_register::define_mmio_register;
use ruspiro_timer as timer;

#[cfg(feature = "ruspiro_pi3")]
const PERIPHERAL_BASE: u32 = 0x3F00_0000;

const I2C_BASE: u32 = PERIPHERAL_BASE + 0x0080_4000; // I²C peripheral register base address
const I2C_MAX_BYTES: usize = 16; // max FiFo size of the I²C peripheral
const I2C_DEFAULT_WAIT: u32 = 2000; // max cycles to wait for a device to acknowledge a request

use crate::I2cResult;

pub(crate) fn initialize(core_speed: u32, fast_mode: bool) -> I2cResult<()> {
    // when I2C is about to be initialized reserve GPIO Pins 2 and 3
    // as the I2C bus pins with alt function 0
    GPIO.take_for(|gpio| {
        let _ = gpio.get_pin(2).map(|pin| pin.into_alt_f0());
        let _ = gpio.get_pin(3).map(|pin| pin.into_alt_f0());
        Ok(())
    }).and_then(|_| {
        // both pin's configured, now setup the I2C speed and we are done
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
        I2C_REG_S::Register.write_value(
            I2C_REG_S::CLK_TIMEOUT::SET | I2C_REG_S::ACK_ERROR::SET | I2C_REG_S::TRANS_DONE::SET,
        );
        I2C_REG_C::Register.write_value(
            I2C_REG_C::ENABLE::SET
                | I2C_REG_C::STARTTRANS::SET
                | I2C_REG_C::FIFO_CLR::CLEAR
                | I2C_REG_C::READWRITE::READ,
        );

        if wait_i2c_done(100).is_ok() {
            r.push(addr as u8);
        };
    }

    r
}

pub(crate) fn check_device(addr: u8) -> I2cResult<()> {
    I2C_REG_A::Register.set(addr as u32);
    I2C_REG_DLEN::Register.set(1);
    I2C_REG_S::Register.write_value(
        I2C_REG_S::CLK_TIMEOUT::SET | I2C_REG_S::ACK_ERROR::SET | I2C_REG_S::TRANS_DONE::SET,
    );
    I2C_REG_C::Register.write_value(
        I2C_REG_C::ENABLE::SET
            | I2C_REG_C::STARTTRANS::SET
            | I2C_REG_C::FIFO_CLR::CLEAR
            | I2C_REG_C::READWRITE::READ,
    );

    wait_i2c_done(100)
}

pub(crate) fn read_reg_u8(addr: u8, reg: u8) -> I2cResult<u8> {
    // reading I²C device regiser data means:
    // 1. write the register address to the device and wait for acknowledge
    // 2. read from the device and wait for acknowledge
    // 3. data available in the fifo
    write_register(addr, reg)?;
    I2C_REG_DLEN::Register.set(1);
    I2C_REG_S::Register.write_value(
        I2C_REG_S::CLK_TIMEOUT::SET | I2C_REG_S::ACK_ERROR::SET | I2C_REG_S::TRANS_DONE::SET,
    );
    I2C_REG_C::Register.write_value(
        I2C_REG_C::ENABLE::SET
            | I2C_REG_C::STARTTRANS::SET
            | I2C_REG_C::FIFO_CLR::CLEAR
            | I2C_REG_C::READWRITE::READ,
    );
    wait_i2c_done(I2C_DEFAULT_WAIT)?;
    let mut buff: [u8; 1] = [0; 1];
    read_fifo(&mut buff);
    Ok(buff[0])
}

pub(crate) fn read_reg_u16(addr: u8, reg: u8) -> I2cResult<u16> {
    let mut buff: [u8; 2] = [0; 2];
    read_reg_data(addr, reg, &mut buff)?;
    Ok((buff[0] as u16) << 8 | (buff[1] as u16))
}

pub(crate) fn read_reg_data(addr: u8, reg: u8, buffer: &mut [u8]) -> I2cResult<usize> {
    // reading I²C device regiser data means:
    // 1. write the register address to the device and wait for acknowledge
    // 2. read from the device and wait for acknowledge
    // 3. data available in the fifo
    write_register(addr, reg)?;
    I2C_REG_DLEN::Register.set(buffer.len() as u32);
    I2C_REG_S::Register.write_value(
        I2C_REG_S::CLK_TIMEOUT::SET | I2C_REG_S::ACK_ERROR::SET | I2C_REG_S::TRANS_DONE::SET,
    );
    I2C_REG_C::Register.write_value(
        I2C_REG_C::ENABLE::SET
            | I2C_REG_C::STARTTRANS::SET
            | I2C_REG_C::FIFO_CLR::CLEAR
            | I2C_REG_C::READWRITE::READ,
    );
    wait_i2c_done(I2C_DEFAULT_WAIT)?;
    //let mut data: Vec<u8> = Vec::with_capacity(count as usize);
    let chunks = buffer.len() / I2C_MAX_BYTES;
    let mut remainder = buffer.len();
    for c in 0..chunks + 1 {
        let start = c * I2C_MAX_BYTES;
        let size = if remainder > I2C_MAX_BYTES {
            I2C_MAX_BYTES
        } else {
            remainder
        };
        read_fifo(&mut buffer[start..start + size]);
        remainder -= I2C_MAX_BYTES;
    }
    Ok(buffer.len())
}

pub(crate) fn write_raw_u8(addr: u8, data: u8) -> I2cResult<()> {
    // clear status flags
    I2C_REG_S::Register.write_value(
        I2C_REG_S::CLK_TIMEOUT::SET | I2C_REG_S::ACK_ERROR::SET | I2C_REG_S::TRANS_DONE::SET,
    );
    // clear FiFo data in case FiFo data has remained from previous calls
    I2C_REG_C::Register.write(I2C_REG_C::FIFO_CLR, 1);
    // set the slave address we would like to send data to and the register id
    I2C_REG_A::Register.set(addr as u32);
    I2C_REG_DLEN::Register.set(1);
    I2C_REG_FIFO::Register.set(data as u32);
    // transmit the data
    I2C_REG_C::Register.write_value(
        I2C_REG_C::ENABLE::SET | I2C_REG_C::STARTTRANS::SET | I2C_REG_C::READWRITE::WRITE,
    );

    wait_i2c_done(I2C_DEFAULT_WAIT)
}

pub(crate) fn write_reg_u8(addr: u8, reg: u8, data: u8) -> I2cResult<()> {
    // clear status flags
    I2C_REG_S::Register.write_value(
        I2C_REG_S::CLK_TIMEOUT::SET | I2C_REG_S::ACK_ERROR::SET | I2C_REG_S::TRANS_DONE::SET,
    );
    // clear FiFo data in case FiFo data has remained from previous calls
    I2C_REG_C::Register.write(I2C_REG_C::FIFO_CLR, 1);
    // set the slave address we would like to send data to and the register id
    I2C_REG_A::Register.set(addr as u32);
    I2C_REG_DLEN::Register.set(2);
    I2C_REG_FIFO::Register.set(reg as u32);
    I2C_REG_FIFO::Register.set(data as u32);
    // transmit the data
    I2C_REG_C::Register.write_value(
        I2C_REG_C::ENABLE::SET | I2C_REG_C::STARTTRANS::SET | I2C_REG_C::READWRITE::WRITE,
    );

    wait_i2c_done(I2C_DEFAULT_WAIT)
}

pub(crate) fn write_reg_u16(addr: u8, reg: u8, data: u16) -> I2cResult<()> {
    let buffer: [u8; 2] = [(data >> 8) as u8, (data & 0xFF) as u8];
    write_reg_data(addr, reg, &buffer)
}

pub(crate) fn write_reg_data(addr: u8, reg: u8, data: &[u8]) -> I2cResult<()> {
    let mut data_len = data.len();
    // clear status flags
    I2C_REG_S::Register.write_value(
        I2C_REG_S::CLK_TIMEOUT::SET | I2C_REG_S::ACK_ERROR::SET | I2C_REG_S::TRANS_DONE::SET,
    );
    // clear FiFo data in case FiFo data has remained from previous calls
    I2C_REG_C::Register.write(I2C_REG_C::FIFO_CLR, 1);
    // set the slave address we would like to send data to and the register id
    I2C_REG_A::Register.set(addr as u32);
    I2C_REG_DLEN::Register.set((data_len + 1) as u32);
    I2C_REG_FIFO::Register.set(reg as u32);
    // transmit the data
    I2C_REG_C::Register.write_value(
        I2C_REG_C::ENABLE::SET | I2C_REG_C::STARTTRANS::SET | I2C_REG_C::READWRITE::WRITE,
    );
    let chunks = data_len / I2C_MAX_BYTES;
    for chunk in 0..chunks + 1 {
        let idx = chunk * data_len;
        let len = if data_len > I2C_MAX_BYTES {
            I2C_MAX_BYTES
        } else {
            data_len
        };
        write_fifo(&data[idx..len]);
        data_len -= I2C_MAX_BYTES;
    }

    wait_i2c_done(I2C_DEFAULT_WAIT)
}

/// Wait until the current I2C operation has been finished/acknowledged
/// Returns an [Err] in case of a timeout or not beein acknowledged
fn wait_i2c_done(tries: u32) -> I2cResult<()> {
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
fn write_register(addr: u8, reg: u8) -> I2cResult<()> {
    // set the slave address we would like to send data to and the register id
    I2C_REG_A::Register.set(addr.into());
    I2C_REG_DLEN::Register.set(1);
    I2C_REG_FIFO::Register.set(reg.into());
    // transmit the data
    I2C_REG_S::Register.write_value(
        I2C_REG_S::CLK_TIMEOUT::SET | I2C_REG_S::ACK_ERROR::SET | I2C_REG_S::TRANS_DONE::SET,
    );
    I2C_REG_C::Register.write_value(
        I2C_REG_C::ENABLE::SET | I2C_REG_C::STARTTRANS::SET | I2C_REG_C::READWRITE::WRITE,
    );

    wait_i2c_done(I2C_DEFAULT_WAIT)
}

/// Read the data from the I2C FIFO register
fn read_fifo(buffer: &mut [u8]) -> usize {
    //let mut data: Vec<u8> = Vec::with_capacity(count as usize);
    let num = if buffer.len() > I2C_MAX_BYTES {
        I2C_MAX_BYTES
    } else {
        buffer.len()
    };
    for i in 0..num {
        while I2C_REG_S::Register.read(I2C_REG_S::RX_DATA) == 0 {}
        buffer[i] = (I2C_REG_FIFO::Register.get() & 0xFF) as u8;
    }
    num
}

/// Write a data buffer to the FIFO
fn write_fifo(data: &[u8]) {
    for i in 0..data.len() {
        while I2C_REG_S::Register.read(I2C_REG_S::TX_DATA) == 0 {}
        I2C_REG_FIFO::Register.set(data[i] as u32);
    }
}

// I2C register definitions
define_mmio_register!(
    // control register
    I2C_REG_C<ReadWrite<u32>@(I2C_BASE + 0x00)> {
        // I²C bus enabled flag
        ENABLE     OFFSET(15) [
            SET = 1,
            CLEAR = 0
        ],
        // Receive interrupt flag
        IRQ_RX     OFFSET(10) [
            SET = 1,
            CLEAR = 0
        ],
        // Transmit interrupt flag
        IRQ_TX     OFFSET(9) [
            SET = 1,
            CLEAR = 0
        ],
        // Done interrupt flag
        IRQ_DONE   OFFSET(8) [
            SET = 1,
            CLEAR = 0
        ],
        // Start transfer flag
        STARTTRANS OFFSET(7) [
            SET = 1,
            CLEAR = 0
        ],
        // clear fifo buffer
        FIFO_CLR  OFFSET(4) [
            CLEAR = 1,
            KEEP = 0
        ],
        // Read / 0 Write operation
        READWRITE  OFFSET(0) [
            READ = 1,
            WRITE = 0
        ]
    }
);

define_mmio_register!(
    // status register
    I2C_REG_S<ReadWrite<u32>@(I2C_BASE + 0x04)> {
        CLK_TIMEOUT  OFFSET(9) [
            SET = 1,
            CLEAR = 0
        ], // 1 Slave has held the SCL signal longer than allowed high
        ACK_ERROR    OFFSET(8) [
            SET = 1,
            CLEAR = 0
        ], // 1 Slave address acknowledge error
        RX_FULL      OFFSET(7) [
            SET = 1,
            CLEAR = 0
        ], // 1 FIFO is full
        TX_EMPTY     OFFSET(6) [
            SET = 1,
            CLEAR = 0
        ], // 1 FIFO is empty
        RX_DATA      OFFSET(5) [
            SET = 1,
            CLEAR = 0
        ], // 1 FIFO contains at least one byte
        TX_DATA      OFFSET(4) [
            SET = 1,
            CLEAR = 0
        ], // 1 FIFO can accept data
        RX_NEEDREAD  OFFSET(3) [
            SET = 1,
            CLEAR = 0
        ], // 1 FIFO is full and needs reading from the FIFO
        TX_NEEDWRITE OFFSET(2) [
            SET = 1,
            CLEAR = 0
        ], // 1 FIFO is less than full and needs writing to the FIFO
        TRANS_DONE   OFFSET(1) [
            SET = 1,
            CLEAR = 0
        ], // 1 if transfer is complete
        TRANS_ACTIVE OFFSET(0) [
            SET = 1,
            CLEAR = 0
        ]  // 1 if transfer is active
    },
    // data len register
    I2C_REG_DLEN<ReadWrite<u32>@(I2C_BASE + 0x08)> {
        DATA OFFSET(0) BITS(16)
    },
    // slave address register
    I2C_REG_A<ReadWrite<u32>@(I2C_BASE + 0x0C)>,
    // FiFo data register
    I2C_REG_FIFO<ReadWrite<u32>@(I2C_BASE + 0x10)>,
    // clock divisor 
    I2C_REG_CDIV<ReadWrite<u32>@(I2C_BASE + 0x14)>,
    // data delay
    I2C_REG_DEL<ReadWrite<u32>@(I2C_BASE + 0x18)>,
    // clock stretch timeout
    I2C_REG_CLKT<ReadWrite<u32>@(I2C_BASE + 0x1C)>
);
