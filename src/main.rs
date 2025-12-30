//! embassy powered alarm clock
//!
//! This is an example of running the embassy executor with multiple tasks
//! concurrently.

#![no_std]
#![no_main]

mod bt;
mod rtc_ds3231;

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
use static_cell::StaticCell;
use trouble_host::{
    Address, Host, HostResources,
    gap::{GapConfig, PeripheralConfig},
    prelude::*,
};

use crate::{
    bt::ble_bas_peripheral::{Server, ble_runner, init_ble, runthis},
    rtc_ds3231::RtcDS3231,
};

// Found via `espflash`
pub const MAC_ADDR: &'static str = "10:20:ba:91:bb:b4";

pub type I2cAsync = I2c<'static, esp_hal::Async>;

pub type MyController = ExternalController<BleConnector<'static>, 20>;
pub type StackType =
    Stack<'static, ExternalController<BleConnector<'static>, 20>, DefaultPacketPool>;

pub static RADIO_INIT: StaticCell<esp_radio::Controller<'static>> = StaticCell::new();

pub static HOST_RESOURCES: StaticCell<
    HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX>,
> = StaticCell::new();

/// Max number of connections
pub const CONNECTIONS_MAX: usize = 1;

/// Max number of L2CAP channels.
pub const L2CAP_CHANNELS_MAX: usize = 3; // Signal + att + CoC

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
    let i2c: I2cAsync = I2c::new(peripherals.I2C0, i2c::master::Config::default())
        .expect("I2C Failed to Initialize!")
        .with_sda(peripherals.GPIO1)
        .with_scl(peripherals.GPIO2)
        .into_async();

    defmt::info!("Init Alarm...");

    let rtc: RtcDS3231 = rtc_ds3231::init_rtc(i2c).await.unwrap();

    // Using a fixed "random" address can be useful for testing. In real scenarios, one would
    // use e.g. the MAC 6 byte array as the address (how to get that varies by the platform).
    let address: Address = Address::random([0xff, 0x8f, 0x1a, 0x05, 0xe4, 0xff]);
    info!("Our address = {}", address.addr);

    static STACK: StaticCell<StackType> = StaticCell::new();
    info!("Initializing Bluetooth...");
    let resources: &'static mut HostResources<
        DefaultPacketPool,
        CONNECTIONS_MAX,
        L2CAP_CHANNELS_MAX,
    > = HOST_RESOURCES.init(HostResources::new());

    let ble_controller = init_ble(peripherals.WIFI, peripherals.BT);
    info!("Initialized Bluetooth!");

    let stack: &'static mut StackType =
        STACK.init(trouble_host::new(ble_controller, resources).set_random_address(address));

    let Host {
        peripheral, runner, ..
    } = stack.build();

    let server = Server::new_with_config(GapConfig::Peripheral(PeripheralConfig {
        name: "TrouBLE",
        appearance: &appearance::power_device::GENERIC_POWER_DEVICE,
    }))
    .unwrap();

    info!("Running Embassy spawners");
    spawner.must_spawn(ble_runner(runner));

    spawner
        .spawn(runthis(peripheral, server, stack))
        .expect("Failed to runthis");

    #[cfg(debug_assertions)]
    spawner.spawn(rtc_ds3231::get_time(rtc)).unwrap();

    spawner
        .spawn(rtc_ds3231::listen_for_alarm(
            peripherals.GPIO5,
            peripherals.GPIO6,
        ))
        .unwrap_or_else(|_| error!("Failed to listen for alarm"));

    info!("All Systems Go!");
}
