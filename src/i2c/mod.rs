//! # I2C
//! This module provides implementations for initializing and generating I2C buses
//! that can be shared between tasks.

use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use esp_hal::peripherals;

use crate::mk_static;

pub(crate) type I2cAsync = esp_hal::i2c::master::I2c<'static, esp_hal::Async>;
pub(crate) type I2cMutex = Mutex<CriticalSectionRawMutex, I2cAsync>;
pub(crate) type I2cBus = I2cDevice<'static, CriticalSectionRawMutex, I2cAsync>;

/// Initialize the I2C bus and return `N` buses
///
/// # Panics
/// Panics if I2C bus fails to initialize
pub(crate) fn init<const N: usize>(
    i2c_peripheral: peripherals::I2C0<'static>,
    sda_pin: peripherals::GPIO2<'static>,
    scl_pin: peripherals::GPIO3<'static>,
) -> heapless::Vec<I2cBus, N> {
    let i2c =
        esp_hal::i2c::master::I2c::new(i2c_peripheral, esp_hal::i2c::master::Config::default())
            .expect("I2C Failed to Initialize!")
            .with_sda(sda_pin) // Might change later since these are for UART
            .with_scl(scl_pin)
            .into_async();

    let i2c_mutex: &'static I2cMutex =
        mk_static!(I2cMutex; Mutex::<CriticalSectionRawMutex, _>::new(i2c));

    let mut buses: heapless::Vec<I2cBus, N> = const { heapless::Vec::new() };
    for _ in 0..N {
        // SAFETY: N slots are alotted and we only push N times
        unsafe {
            buses.push_unchecked(I2cDevice::new(i2c_mutex));
        }
    }

    buses
}

// /// Given N `usize` and `&I2cMutex`, it will create and return N `MaybeUninit<I2cDevice>`s
// pub(crate) fn get_bus_arr<const N: usize>(
//     mutex: &'static I2cMutex,
// ) -> [core::mem::MaybeUninit<I2cBus>; N] {
//     let mut arr = [const { core::mem::MaybeUninit::uninit() }; N];

//     for bus in &mut arr {
//         bus.write(embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice::new(mutex));
//     }

//     arr
// }
