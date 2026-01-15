//! embassy powered alarm clock
//!
//! This is an example of running the embassy executor with multiple tasks
//! concurrently.

#![no_std]
#![no_main]
// NIGHTLY: Required for Picoserve
#![feature(impl_trait_in_assoc_type)]
// NIGHTLY: Required for `static_cell::make_static!`
#![feature(type_alias_impl_trait)]

mod rtc_ds3231;
mod wireless;

use bt_hci::{controller::ExternalController, uuid::appearance};
use defmt::{error, info};
use embassy_executor::Spawner;
use esp_backtrace as _;
#[cfg(target_arch = "riscv32")]
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::{
    clock::CpuClock,
    i2c::{self, master::I2c},
    timer::timg::TimerGroup,
};
use esp_println as _;
use esp_radio::ble::controller::BleConnector;
use trouble_host::{
    Address, HostResources,
    gap::{GapConfig, PeripheralConfig},
    prelude::DefaultPacketPool,
};

// Found via `espflash`
// pub const MAC_ADDR: &'static str = "10:20:ba:91:bb:b4";
pub const MAC_ADDR: [u8; 6] = [0x10, 0x20, 0xba, 0x91, 0xbb, 0xb4];

pub type I2cAsync = I2c<'static, esp_hal::Async>;

/// Max number of connections for Bluetooth
pub const BLE_CONNECTIONS_MAX: usize = 2;

/// Max number of L2CAP channels.
pub const L2CAP_CHANNELS_MAX: usize = 3; // Signal + att + CoC
pub type BleController = ExternalController<BleConnector<'static>, 20>;
pub type BleResources = HostResources<DefaultPacketPool, BLE_CONNECTIONS_MAX, L2CAP_CHANNELS_MAX>;
pub type BleStack =
    trouble_host::Stack<'static, ExternalController<BleConnector<'static>, 20>, DefaultPacketPool>;

use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, lazy_lock::LazyLock, mutex::Mutex,
};

use embassy_sync::signal::Signal;

use crate::{
    rtc_ds3231::{RTC_DS3231, RtcDS3231, RtcTime},
    wireless::{
        bt::{
            self,
            ble_bas_peripheral::{Server, ble_runner_task},
        },
        init_wireless,
        wifi::{
            connect_to_wifi,
            get_net_stack,
            net_runner_task,
            sntp::fetch_sntp,
            web_server::{WEB_TASK_POOL_SIZE, init_web, web_task},
            // web_server::serve_webpage,
        },
    },
};

pub static TIME_SIGNAL: Signal<CriticalSectionRawMutex, RtcTime> = Signal::new();

pub static EPOCH_SIGNAL: Signal<CriticalSectionRawMutex, i64> = Signal::new();

/// Fires once NTP gets a valid time
pub static NTP_ONESHOT: Signal<CriticalSectionRawMutex, i64> = Signal::new();

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

esp_bootloader_esp_idf::esp_app_desc!();

// When you are okay with using a nightly compiler it's better to use https://docs.rs/static_cell/2.1.0/static_cell/macro.make_static.html
#[macro_export]
macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
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

    defmt::info!("Init Alarm...");
    let rtc: RtcDS3231 = rtc_ds3231::init_rtc(i2c).await.unwrap();

    let rtc: &Mutex<CriticalSectionRawMutex, RtcDS3231> = RTC_DS3231.init(Mutex::new(rtc));

    // Using a fixed "random" address can be useful for testing. In real scenarios, one would
    // use e.g. the MAC 6 byte array as the address (how to get that varies by the platform).
    let address: Address = Address::random(MAC_ADDR);
    info!("Our address = {}", address.addr);

    info!("Initializing Wireless...");

    let (wifi_controller, wifi_interface, ble_controller) =
        init_wireless(peripherals.WIFI, peripherals.BT);

    let (net_stack, net_runner) = get_net_stack(wifi_interface);
    // let (ble_stack, ble_runner) = get_ble_stack();

    let ble_resources = mk_static!(BleResources, HostResources::new());
    let ble_stack: &'static mut BleStack = mk_static!(
        BleStack,
        trouble_host::new(ble_controller, ble_resources).set_random_address(address)
    );

    let ble_host = ble_stack.build();
    let ble_peripheral = ble_host.peripheral;
    let ble_runner = ble_host.runner;

    let gatt_server = Server::new_with_config(GapConfig::Peripheral(PeripheralConfig {
        name: "TrouBLE",
        appearance: &appearance::power_device::GENERIC_POWER_DEVICE,
    }))
    .unwrap();
    info!("Initialized Wireless!");

    info!("Running Embassy spawners");
    spawner.must_spawn(ble_runner_task(ble_runner));
    spawner.must_spawn(net_runner_task(net_runner));
    spawner.must_spawn(connect_to_wifi(wifi_controller));

    spawner.must_spawn(bt::run_peripheral(ble_peripheral, gatt_server, ble_stack));
    spawner.must_spawn(fetch_sntp(net_stack, rtc));

    spawner.must_spawn(rtc_ds3231::run(rtc));
    spawner.must_spawn(rtc_ds3231::update_rtc_timestamp(rtc));

    spawner
        .spawn(rtc_ds3231::listen_for_alarm(
            peripherals.GPIO5,
            peripherals.GPIO6,
        ))
        .unwrap_or_else(|_| error!("Failed to listen for alarm"));

    let (app, conf) = init_web();

    for task_id in 0..WEB_TASK_POOL_SIZE {
        spawner.must_spawn(web_task(task_id, net_stack, app, conf));
    }

    info!("All Systems Go!");
    info!("Running.... ");
}
