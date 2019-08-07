# RusPiRo I²C Bus interface crate


[![Travis-CI Status](https://api.travis-ci.org/RusPiRo/ruspiro-i2c.svg?branch=master)](https://travis-ci.org/RusPiRo/ruspiro-i2c)
[![Latest Version](https://img.shields.io/crates/v/ruspiro-i2c.svg)](https://crates.io/crates/ruspiro-i2c)
[![Documentation](https://docs.rs/ruspiro-i2c/badge.svg)](https://docs.rs/ruspiro-<crate>)
[![License](https://img.shields.io/crates/l/ruspiro-i2c.svg)](https://github.com/RusPiRo/ruspiro-i2c#license)


## Usage
To use the crate just add the following dependency to your ``Cargo.toml`` file:
```
[dependencies]
ruspiro-i2c = "0.1.0"
```

Once done the access to the I²C bus interface is available in your rust files like so:
```
use ruspiro_i2c::*;

```

## License
Licensed under Apache License, Version 2.0, ([LICENSE](LICENSE) or http://www.apache.org/licenses/LICENSE-2.0)