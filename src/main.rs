//! # Rusty-Clock
//! An Embassy powered alarm clock
//!
//! WARN: I accidentally shorted out my GPIO9, destroying the LED but somehow it still works??
//! I'm assuming the LED acted like a fuse, however the GPIO9 might be damaged in some way
//! that I am not aware of

#![no_std]
#![no_main]
#![recursion_limit = "256"]
#![feature(allocator_api)]
#![feature(decl_macro)]
// NIGHTLY: Required for Picoserve
#![feature(impl_trait_in_assoc_type)]
// NIGHTLY: Required for `static_cell::make_static!`
#![feature(type_alias_impl_trait)]
// NIGHTLY: Allows env vars to be parsed at compile time
#![feature(const_option_ops)]
#![feature(const_trait_impl)]
#![feature(const_result_trait_fn)]
#![feature(const_result_unwrap_unchecked)]
// NIGHTLY: Enum-based typestate pattern
#![feature(adt_const_params)]

extern crate alloc;

mod buzzer;
mod i2c;
mod pwm;
mod rtc_ds3231;
mod utils;
mod wireless;

use defmt_rtt as _;
use esp_backtrace as _;
// use esp_println as _;

use defmt::info;
use embassy_executor::Spawner;
#[cfg(target_arch = "riscv32")]
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::{
    clock::CpuClock,
    gpio::{Output, OutputConfig},
    timer::timg::TimerGroup,
};

// Found via `espflash`
// pub const MAC_ADDR: &'static str = "10:20:ba:91:bb:b4";
pub(crate) const MAC_ADDR: [u8; 6] = [0x10, 0x20, 0xba, 0x91, 0xbb, 0xb4];

// TIP: Set these in .env if using direnv
pub(crate) const SSID: &str = env!("SSID");
pub(crate) const PASSWORD: &str = env!("PASSWORD");

// NOTE: Using TZ_OFFSET since IANA Timezones adds unnecessary weight
// PERF: Faster and leaner than LazyLock if you're
// okay with using unsafe and nightly features
pub(crate) const TZ_OFFSET: i8 = {
    let tz = option_env!("TZ_OFFSET").unwrap_or("0");

    // SAFETY: Caller is required to guarantee valid number
    // NOTE: Result::unwrap cannot be used since it is non-const
    unsafe { i8::from_str_radix(tz, 10).unwrap_unchecked() }
};

// TEST: Within UTC Offset range
static_assertions::const_assert!(TZ_OFFSET <= 12 && TZ_OFFSET >= -12);
esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[unsafe(link_section = ".dram2_uninit")] size: 66320);
    // COEX needs more RAM - so we've added some more
    esp_alloc::heap_allocator!(size: 64 * 1024);

    #[cfg(target_arch = "riscv32")]
    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(
        timg0.timer0,
        #[cfg(target_arch = "riscv32")]
        sw_int.software_interrupt0,
    );

    info!("ESP-RTOS Started!");

    info!("Initializing I2C...");
    let mut i2c_buses: heapless::Vec<i2c::I2cBus, 2> =
        i2c::init(peripherals.I2C0, peripherals.GPIO2, peripherals.GPIO3);
    let i2c_rtc = unsafe { i2c_buses.pop_unchecked() };

    info!("Init RTC...");
    rtc_ds3231::init(spawner, i2c_rtc).await;

    info!("Init PWM/LEDC");
    let output = Output::new(
        peripherals.GPIO5,
        esp_hal::gpio::Level::Low,
        OutputConfig::default().with_drive_strength(esp_hal::gpio::DriveStrength::_5mA),
    );

    let chan = pwm::init(peripherals.LEDC, output).channel0();

    info!("Init Buzzer...");
    buzzer::init(spawner, chan, peripherals.GPIO7, peripherals.GPIO6).await;

    info!("Initializing Wireless...");
    wireless::init(spawner, peripherals.WIFI, peripherals.BT);

    info!("All Systems Go!");
    info!("Running.... ");
}
