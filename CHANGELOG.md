# Changelog

## :melon: v0.4.0

- ### :wrench: Maintenance

  - build the single crate with `aarch64-unknown-none` target
  - update dependent crates to latest version
  - ensure successful build with latest nightly (2021-09-05) version

## :banana: v0.3.1

- ### :wrench: Maintenance

  - update dependent crate versions
  - stabilize build with `cargo make`

## :carrot: v0.3.0

- ### :bulb: Features

  Enable Aarch64 build target architecture
- ### :wrench: Maintenance

  Remove the I2CDevice structure until there is a better abstraction how a I2C device might be
  encapsulated for use and respective device driver implementing crates
  