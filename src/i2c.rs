//! # I2C
//! This module provides implementations for initializing and generating I2C buses
//! that can be shared between tasks.

use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use esp_hal::{
    i2c::master::{Config, I2c},
    peripherals,
};

use crate::utils::mk_static;

type I2cAsync = esp_hal::i2c::master::I2c<'static, esp_hal::Async>;
type I2cMutex = Mutex<CriticalSectionRawMutex, I2cAsync>;
pub(crate) type I2cBus = I2cDevice<'static, CriticalSectionRawMutex, I2cAsync>;

/// Initializes the I2C bus and returns an array of `N` buses.
///
/// # Panics
/// Panics if I2C bus fails to initialize.
pub(crate) fn init<const N: usize>(
    i2c_peripheral: peripherals::I2C0<'static>,
    sda_pin: peripherals::GPIO2<'static>,
    scl_pin: peripherals::GPIO3<'static>,
) -> [I2cBus; N] {
    let i2c = I2c::new(i2c_peripheral, Config::default())
        .expect("I2C Failed to Initialize")
        .with_sda(sda_pin)
        .with_scl(scl_pin)
        .into_async();

    let i2c_mutex: &'static I2cMutex = mk_static!(I2cMutex; Mutex::new(i2c));
    core::array::from_fn(|_| I2cDevice::new(i2c_mutex))
}

#[cfg(debug_assertions)]
#[expect(unused, reason = "This only used for diagnostics")]
pub(crate) async fn scan_i2c_addrs(mut i2c: impl embedded_hal_async::i2c::I2c) {
    for address in 1..128 {
        if let Ok(()) = i2c.write(address, &[]).await {
            defmt::println!("Device found at address 0x{:02x}", address);
        }
    }
}
