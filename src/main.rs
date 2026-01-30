//! embassy powered alarm clock
//!
//! This is an example of running the embassy executor with multiple tasks
//! concurrently.

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

extern crate alloc;

mod buzzer;
mod rtc_ds3231;
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
    i2c::{self, master::I2c},
    timer::timg::TimerGroup,
};

// Found via `espflash`
// pub const MAC_ADDR: &'static str = "10:20:ba:91:bb:b4";
pub const MAC_ADDR: [u8; 6] = [0x10, 0x20, 0xba, 0x91, 0xbb, 0xb4];

use crate::{
    buzzer::init_buzzer,
    rtc_ds3231::{TIME_WATCH, init_rtc},
    wireless::{
        bt::{self, BleStack, gatt::Server},
        init_wireless,
    },
};

// TIP: Set these in .env if using direnv
pub const SSID: &str = env!("SSID");
pub const PASSWORD: &str = env!("PASSWORD");

// NOTE: Using TZ_OFFSET since IANA Timezones adds unnecessary weight
// PERF: Faster and leaner than LazyLock if you're
// okay with using unsafe and nightly features
pub const TZ_OFFSET: i8 = {
    let tz = option_env!("TZ_OFFSET").unwrap_or("0");

    // SAFETY: Caller is required to guarantee valid number
    // NOTE: Result::unwrap cannot be used since it is non-const
    unsafe { i8::from_str_radix(tz, 10).unwrap_unchecked() }
};

// TEST: Within UTC Offset range
static_assertions::const_assert!(TZ_OFFSET <= 12 && TZ_OFFSET >= -12);
esp_bootloader_esp_idf::esp_app_desc!();

/// Convert a `T` to a `&'static mut T`.
///
/// The macro declares a `static StaticCell` and then initializes it when run, returning the `&'static mut`.
/// Therefore, each instance can only be run once. Next runs will panic. The `static` can additionally be
/// decorated with attributes, such as `#[link_section]`, `#[used]`, et al.
pub macro mk_static {
    ($t:ty; $val:expr) => (mk_static!($t, $val, )),
    ($t:ty, $val:expr) => (mk_static!($t, $val, )),
    ($t:ty, $val:expr, $(#[$m:meta])*) => {{
        $(#[$m])*
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write($val);
        x
    }}
}

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
    let i2c = I2c::new(peripherals.I2C0, i2c::master::Config::default())
        .expect("I2C Failed to Initialize!")
        .with_sda(peripherals.GPIO2) // Might change later since these are for UART
        .with_scl(peripherals.GPIO3)
        .into_async();

    info!("Init RTC...");
    init_rtc(spawner, i2c).await;

    info!("Init Buzzer...");
    init_buzzer(spawner, peripherals.GPIO5, peripherals.GPIO7).await;

    info!("Initializing Wireless...");
    init_wireless(spawner, peripherals.WIFI, peripherals.BT);

    info!("All Systems Go!");
    info!("Running.... ");
}
