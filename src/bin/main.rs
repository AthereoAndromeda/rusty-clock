//! embassy powered alarm clock
//!
//! This is an example of running the embassy executor with multiple tasks
//! concurrently.

#![no_std]
#![no_main]

mod rtc;

use ds3231::DS3231;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
#[cfg(target_arch = "riscv32")]
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::{
    gpio::{DriveStrength, Input, InputConfig, Level, Output, OutputConfig, Pull},
    i2c::{self, master::I2c},
    timer::timg::TimerGroup,
};
use esp_println as _;

pub(crate) type I2cAsync = I2c<'static, esp_hal::Async>;
pub(crate) type RTC = DS3231<I2cAsync>;
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

    #[cfg(target_arch = "riscv32")]
    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(
        timg0.timer0,
        #[cfg(target_arch = "riscv32")]
        sw_int.software_interrupt0,
    );

    let i2c: I2cAsync = I2c::new(peripherals.I2C0, i2c::master::Config::default())
        .unwrap()
        .with_sda(peripherals.GPIO1)
        .with_scl(peripherals.GPIO2)
        .into_async();

    defmt::info!("Init Alarm...");
    let alarm_input = Input::new(
        peripherals.GPIO6,
        InputConfig::default().with_pull(Pull::None),
    );

    let mut buzzer_output = Output::new(
        peripherals.GPIO5,
        Level::High,
        OutputConfig::default().with_drive_strength(DriveStrength::_5mA),
    );

    // Beep 3 times
    esp_hal::delay::Delay::new().delay_millis(300);
    buzzer_output.toggle();
    esp_hal::delay::Delay::new().delay_millis(300);
    buzzer_output.toggle();
    esp_hal::delay::Delay::new().delay_millis(300);
    buzzer_output.toggle();

    let rtc: RTC = rtc::init_rtc(i2c).await.unwrap();

    #[cfg(debug_assertions)]
    spawner.spawn(rtc::get_time(rtc)).unwrap();

    spawner
        .spawn(rtc::listen_for_alarm(buzzer_output, alarm_input))
        .unwrap();
}
