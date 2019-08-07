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

use ruspiro_singleton::Singleton;
use ruspiro_console::*;

mod interface;

/// static singleton accessor for the I²C bus peripheral
pub static I2C: Singleton<I2c> = Singleton::new(I2c::new());

/// I²C peripheral representation
pub struct I2c {
    initialized: bool,
}

impl I2c {
    pub(crate) const fn new() -> Self {
        I2c {
            initialized: false,
        }
    }
}