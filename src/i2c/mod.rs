use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use esp_hal::peripherals;

use crate::mk_static;

pub(crate) type I2cAsync = esp_hal::i2c::master::I2c<'static, esp_hal::Async>;
pub(crate) type I2cMutex = Mutex<CriticalSectionRawMutex, I2cAsync>;
pub(crate) type I2cBus = I2cDevice<'static, CriticalSectionRawMutex, I2cAsync>;

pub fn init_i2c<const N: usize>(
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

    let i2c_mutex: &'static I2cMutex = mk_static!(Mutex<CriticalSectionRawMutex, I2cAsync>; Mutex::<CriticalSectionRawMutex, _>::new(i2c));
    create_buses(i2c_mutex)
}

/// Given N `usize` and `&I2cMutex`, it will create and return N `MaybeUninit<I2cDevice>`s
/// # Safety
/// This macro is unsafe
pub macro get_bus_arr($num:expr; $mutex:expr) {
    let mut arr: [MaybeUninit<$crate::i2c::I2cBus>; $num] = [const { core::mem::zeroed() }; $num];

    for bus in arr {
        bus.write(embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice::new($mutex));
    }

    arr
}

pub fn create_buses<const N: usize>(mutex: &'static I2cMutex) -> heapless::Vec<I2cBus, N> {
    let mut v: heapless::Vec<I2cBus, N> = const { heapless::Vec::new() };

    for _ in 0..N {
        // SAFETY: N slots are alotted and we only push N times
        unsafe {
            v.push_unchecked(I2cDevice::new(mutex));
        }
    }

    v
}
