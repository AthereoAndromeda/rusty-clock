//! embassy powered alarm clock
//!
//! This is an example of running the embassy executor with multiple tasks
//! concurrently.

#![no_std]
#![no_main]
#![recursion_limit = "256"]
// NIGHTLY: Required for Picoserve
#![feature(impl_trait_in_assoc_type)]
// NIGHTLY: Required for `static_cell::make_static!`
#![feature(type_alias_impl_trait)]
#![feature(allocator_api)]

extern crate alloc;

mod buzzer;
mod rtc_ds3231;
mod wireless;

use defmt_rtt as _;
use esp_backtrace as _;
// use esp_println as _;

use defmt::{error, info};
use embassy_executor::Spawner;
#[cfg(target_arch = "riscv32")]
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::{
    clock::CpuClock,
    gpio::Output,
    i2c::{self, master::I2c},
    timer::timg::TimerGroup,
};

// Found via `espflash`
// pub const MAC_ADDR: &'static str = "10:20:ba:91:bb:b4";
pub const MAC_ADDR: [u8; 6] = [0x10, 0x20, 0xba, 0x91, 0xbb, 0xb4];

pub type I2cAsync = I2c<'static, esp_hal::Async>;

use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, lazy_lock::LazyLock, mutex::Mutex,
};

use embassy_sync::signal::Signal;

use crate::{
    buzzer::init_buzzer,
    rtc_ds3231::{RTC_DS3231, RtcDS3231, rtc_time::RtcTime},
    wireless::{
        bt::{self, BleStack, ble_runner_task, gatt::Server, get_ble_stack},
        init_wireless,
        wifi::{
            connect_to_wifi, get_net_stack, net_runner_task,
            sntp::fetch_sntp,
            web_server::{WEB_TASK_POOL_SIZE, init_web, web_task},
        },
    },
};

pub static TIME_SIGNAL: Signal<CriticalSectionRawMutex, RtcTime> = Signal::new();

// TIP: Set these in .env if using direnv
pub const SSID: &str = env!("SSID");
pub const PASSWORD: &str = env!("PASSWORD");

// NOTE: Using TZ_OFFSET since IANA Timezones adds unnecessary weight
pub static TZ_OFFSET: LazyLock<i8> = LazyLock::new(|| {
    option_env!("TZ_OFFSET")
        .unwrap_or("0")
        .parse::<i8>()
        .expect("Must be a valid i8!")
});

pub const NTP_SERVER_ADDR: &str = "pool.ntp.org";

pub type BuzzerOutput = Mutex<CriticalSectionRawMutex, Output<'static>>;
esp_bootloader_esp_idf::esp_app_desc!();

// When you are okay with using a nightly compiler it's better to use https://docs.rs/static_cell/2.1.0/static_cell/macro.make_static.html
#[macro_export]
macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write($val);
        x
    }};
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
    let i2c: I2cAsync = I2c::new(peripherals.I2C0, i2c::master::Config::default())
        .expect("I2C Failed to Initialize!")
        .with_sda(peripherals.GPIO2) // Might change later since these are for UART
        .with_scl(peripherals.GPIO3)
        .into_async();

    info!("Init RTC...");
    let rtc: RtcDS3231 = rtc_ds3231::init_rtc(i2c).await.unwrap();
    let rtc: &Mutex<CriticalSectionRawMutex, RtcDS3231> = RTC_DS3231.init(Mutex::new(rtc));

    info!("Init Buzzer...");
    let buzzer_out = init_buzzer(peripherals.GPIO5);

    info!("Initializing Wireless...");
    let (wifi_controller, wifi_interface, ble_controller) =
        init_wireless(peripherals.WIFI, peripherals.BT);

    let (net_stack, net_runner) = get_net_stack(wifi_interface);
    let (ble_stack, ble_host, gatt_server) = get_ble_stack(ble_controller);
    info!("Initialized Wireless!");

    info!("Running Embassy spawners");
    spawner.must_spawn(ble_runner_task(ble_host.runner));
    spawner.must_spawn(net_runner_task(net_runner));
    spawner.must_spawn(connect_to_wifi(wifi_controller));

    spawner.must_spawn(bt::run_peripheral(
        ble_host.peripheral,
        gatt_server,
        ble_stack,
    ));
    spawner.must_spawn(fetch_sntp(net_stack, rtc));

    spawner.must_spawn(rtc_ds3231::run(rtc));

    spawner
        .spawn(rtc_ds3231::listen_for_alarm(peripherals.GPIO6))
        .unwrap_or_else(|_| error!("Failed to listen for alarm"));

    spawner.must_spawn(buzzer::run(buzzer_out));
    spawner.must_spawn(buzzer::listen_for_button(peripherals.GPIO7));
    spawner.must_spawn(buzzer::listen_for_timer());

    let (app, conf) = init_web();

    for task_id in 0..WEB_TASK_POOL_SIZE {
        spawner.must_spawn(web_task(task_id, net_stack, app, conf));
    }

    info!("All Systems Go!");
    info!("Running.... ");
}
