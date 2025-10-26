//! embassy powered alarm clock
//!
//! This is an example of running the embassy executor with multiple tasks
//! concurrently.

#![no_std]
#![no_main]

mod rtc;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
#[cfg(target_arch = "riscv32")]
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::{
    gpio::{AnyPin, Level, Output, Pin},
    i2c::{self, master::I2c},
    timer::timg::TimerGroup,
};
use esp_println as _;

pub(crate) type I2cAsync = I2c<'static, esp_hal::Async>;
pub(crate) const RTC_I2C_ADDR: u8 = 0x68;

esp_bootloader_esp_idf::esp_app_desc!();

#[embassy_executor::task]
async fn blink(mut pin: Output<'static>) {
    loop {
        defmt::info!("Blinked!");
        Timer::after(Duration::from_millis(5000)).await;
        pin.toggle();
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let i2c: I2cAsync = I2c::new(peripherals.I2C0, i2c::master::Config::default())
        .unwrap()
        .with_sda(peripherals.GPIO1)
        .with_scl(peripherals.GPIO2)
        .into_async();

    defmt::info!("Init!");

    #[cfg(target_arch = "riscv32")]
    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(
        timg0.timer0,
        #[cfg(target_arch = "riscv32")]
        sw_int.software_interrupt0,
    );

    let pin = peripherals.GPIO5;
    let out_pin = Output::new(pin, Level::Low, Default::default());

    spawner.spawn(blink(out_pin)).unwrap();
    // spawner.spawn(init_ds3231(i2c)).unwrap();
    spawner.spawn(rtc::get_time(i2c)).unwrap();
}
