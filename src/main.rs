//! embassy powered alarm clock
//!
//! This is an example of running the embassy executor with multiple tasks
//! concurrently.

#![no_std]
#![no_main]

mod bt;
mod rtc_ds3231;

use core::net::{IpAddr, SocketAddr};

use bt_hci::{controller::ExternalController, uuid::appearance};
use defmt::{error, info, println, warn};
use embassy_executor::Spawner;
use embassy_net::{
    StackResources,
    udp::{PacketMetadata, UdpSocket},
};
use embassy_time::Timer;
use esp_backtrace as _;
#[cfg(target_arch = "riscv32")]
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::{
    clock::CpuClock,
    i2c::{self, master::I2c},
    rng::Rng,
    timer::timg::TimerGroup,
};
use esp_println as _;
use esp_radio::{
    ble::controller::BleConnector,
    wifi::{ClientConfig, ModeConfig, WifiController, WifiDevice, WifiEvent, WifiStaState},
};
use jiff::tz::{Offset, TimeZone};
use sntpc::NtpContext;
use static_cell::StaticCell;
use trouble_host::{
    Address, HostResources,
    gap::{GapConfig, PeripheralConfig},
    prelude::DefaultPacketPool,
};

use crate::{
    bt::ble_bas_peripheral::{Server, ble_runner_task, init_wireless},
    rtc_ds3231::{RtcDS3231, RtcTime},
};

// Found via `espflash`
// pub const MAC_ADDR: &'static str = "10:20:ba:91:bb:b4";
pub const MAC_ADDR: [u8; 6] = [0x10, 0x20, 0xba, 0x91, 0xbb, 0xb4];

pub type I2cAsync = I2c<'static, esp_hal::Async>;

pub type BleController = ExternalController<BleConnector<'static>, 20>;
pub type BleResources = HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX>;
pub type BleStack =
    trouble_host::Stack<'static, ExternalController<BleConnector<'static>, 20>, DefaultPacketPool>;

pub static HOST_RESOURCES: StaticCell<BleResources> = StaticCell::new();

/// Max number of connections
pub const CONNECTIONS_MAX: usize = 1;

/// Max number of L2CAP channels.
pub const L2CAP_CHANNELS_MAX: usize = 3; // Signal + att + CoC

pub static RTC_DS3231: StaticCell<Mutex<CriticalSectionRawMutex, RtcDS3231>> = StaticCell::new();

use embassy_sync::channel::Channel;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};

pub static TIME_CH: Channel<CriticalSectionRawMutex, RtcTime, 1> = Channel::new();

use embassy_sync::signal::Signal;

pub static TIME_SIGNAL: Signal<CriticalSectionRawMutex, RtcTime> = Signal::new();

pub static EPOCH_SIGNAL: Signal<CriticalSectionRawMutex, i64> = Signal::new();

/// Fires once NTP gets a valid time
pub static NTP_SIGNAL: Signal<CriticalSectionRawMutex, i64> = Signal::new();

// TIP: Set these in .env if using direnv
pub const SSID: &str = env!("SSID");
pub const PASSWORD: &str = env!("PASSWORD");
pub const TZ: &str = env!("TIMEZONE");

pub const NTP_SERVER: &str = "pool.ntp.org";

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

#[derive(Copy, Clone)]
/// Time in us
pub struct SntpTimestamp(u64);

impl sntpc::NtpTimestampGenerator for SntpTimestamp {
    fn init(&mut self) {}
    fn timestamp_sec(&self) -> u64 {
        self.0 / 1_000_000
    }
    fn timestamp_subsec_micros(&self) -> u32 {
        (self.0 % 1_000_000) as u32
    }
}

impl Default for SntpTimestamp {
    fn default() -> Self {
        Self(0)
    }
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
        .with_sda(peripherals.GPIO1) // Might change later since these are for UART
        .with_scl(peripherals.GPIO2)
        .into_async();

    defmt::info!("Init Alarm...");
    let rtc: RtcDS3231 = rtc_ds3231::init_rtc(i2c).await.unwrap();

    let rtc: &Mutex<CriticalSectionRawMutex, RtcDS3231> = RTC_DS3231.init(Mutex::new(rtc));

    // Using a fixed "random" address can be useful for testing. In real scenarios, one would
    // use e.g. the MAC 6 byte array as the address (how to get that varies by the platform).
    let address: Address = Address::random(MAC_ADDR);
    info!("Our address = {}", address.addr);

    info!("Initializing Bluetooth...");
    let ble_resources: &'static mut HostResources<
        DefaultPacketPool,
        CONNECTIONS_MAX,
        L2CAP_CHANNELS_MAX,
    > = HOST_RESOURCES.init(HostResources::new());

    let (wifi_controller, wifi_interface, ble_controller) =
        init_wireless(peripherals.WIFI, peripherals.BT);
    info!("Initialized Wireless!");

    let (net_stack, net_runner) = get_net_stack(wifi_interface);
    // let (ble_stack, ble_runner) = get_ble_stack();

    info!("Initialized Bluetooth!");

    let ble_stack = mk_static!(
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

    info!("Running Embassy spawners");
    spawner.must_spawn(ble_runner_task(ble_runner));
    spawner.must_spawn(net_runner_task(net_runner));
    spawner.must_spawn(connection(wifi_controller));

    spawner
        .spawn(bt::run_peripheral(ble_peripheral, gatt_server, ble_stack))
        .expect("Failed to run bluetooth peripheral");

    spawner
        .spawn(rtc_ds3231::run(rtc))
        .expect("Unable to get time");

    spawner
        .spawn(rtc_ds3231::update_rtc(rtc))
        .expect("Unable to get time");

    spawner
        .spawn(rtc_ds3231::listen_for_alarm(
            peripherals.GPIO5,
            peripherals.GPIO6,
        ))
        .unwrap_or_else(|_| error!("Failed to listen for alarm"));

    info!("All Systems Go!");

    info!("Running.... ");
    // let mut rx_buffer = [0; 4096];
    // let mut tx_buffer = [0; 4096];

    // Create UDP socket
    // Buffers are statically allocated
    let udp_rx_meta = mk_static!([PacketMetadata; 16], [PacketMetadata::EMPTY; 16]);
    let udp_tx_meta = mk_static!([PacketMetadata; 16], [PacketMetadata::EMPTY; 16]);
    let udp_tx_buffer = mk_static!([u8; 4096], [0u8; 4096]);
    let udp_rx_buffer = mk_static!([u8; 4096], [0u8; 4096]);

    let mut udp_socket = UdpSocket::new(
        net_stack,
        udp_rx_meta,
        udp_rx_buffer,
        udp_tx_meta,
        udp_tx_buffer,
    );

    // 123 is SNTP port
    udp_socket.bind(123).unwrap();

    info!("Waiting for Network Link...");
    loop {
        if net_stack.is_link_up() {
            info!("Network Link is Up!");
            break;
        }
        Timer::after_millis(500).await;
    }

    println!("Waiting to get IP address...");
    loop {
        if let Some(config) = net_stack.config_v4() {
            info!("Got IP: {}", config.address);
            break;
        }
        Timer::after_millis(500).await;
    }

    let ntp_addrs = net_stack
        .dns_query(NTP_SERVER, smoltcp::wire::DnsQueryType::A)
        .await
        .unwrap();

    if ntp_addrs.is_empty() {
        panic!("Failed to resolve DNS. Empty result");
    }

    spawner.must_spawn(fetch_sntp(ntp_addrs[0].into(), udp_socket));
}

fn get_net_stack(
    wifi_interface: esp_radio::wifi::Interfaces<'_>,
) -> (
    embassy_net::Stack<'static>,
    embassy_net::Runner<'_, WifiDevice<'_>>,
) {
    let wifi_interface_station = wifi_interface.sta;

    let rng = Rng::new();
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    let embassy_config = embassy_net::Config::dhcpv4(Default::default());

    // Init network stack
    embassy_net::new(
        wifi_interface_station,
        embassy_config,
        mk_static!(StackResources<3>, StackResources::<3>::new()),
        seed,
    )
}

fn get_ble_stack() {}

#[embassy_executor::task]
async fn net_runner_task(mut runner: embassy_net::Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}

#[embassy_executor::task]
/// Connect to Wi-Fi
async fn connection(mut controller: WifiController<'static>) {
    println!("start connection task");
    println!("Device capabilities: {:?}", controller.capabilities());
    loop {
        match esp_radio::wifi::sta_state() {
            WifiStaState::Connected => {
                // wait until we're no longer connected
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                Timer::after_millis(5000).await
            }
            _ => {}
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let station_config = ModeConfig::Client(
                ClientConfig::default()
                    .with_ssid(SSID.into())
                    .with_password(PASSWORD.into()),
            );
            controller.set_config(&station_config).unwrap();
            println!("Starting wifi");
            controller.start_async().await.unwrap();
            println!("Wifi started!");

            println!("Scan");
            let scan_config = esp_radio::wifi::ScanConfig::default().with_max(10);
            let result = controller
                .scan_with_config_async(scan_config)
                .await
                .unwrap();
            for ap in result {
                println!("{:?}", ap);
            }
        }
        println!("About to connect...");

        match controller.connect_async().await {
            Ok(_) => println!("Wifi connected!"),
            Err(e) => {
                println!("Failed to connect to wifi: {:?}", e);
                Timer::after_millis(5000).await
            }
        }
    }
}

#[embassy_executor::task]
async fn fetch_sntp(addr: IpAddr, udp_socket: UdpSocket<'static>) {
    info!("[sntp] Sending SNTP Request...");
    // let addr: IpAddr = ntp_addrs[0].into();

    let result = sntpc::get_time(
        SocketAddr::from((addr, 123)),
        &udp_socket,
        NtpContext::new(SntpTimestamp(0)),
    )
    .await;

    info!("[sntp] Received a response!");
    match result {
        Ok(time) => {
            info!("Response: {:?}", time);
            let jt = jiff::Timestamp::from_second(time.sec() as i64)
                .unwrap()
                .checked_add(
                    jiff::Span::new()
                        .nanoseconds((time.seconds_fraction as i64 * 1_000_000_000) >> 32),
                )
                .unwrap()
                .to_zoned(TimeZone::fixed(Offset::from_hours(8).unwrap()));

            NTP_SIGNAL.signal(jt.timestamp().as_second());

            #[cfg(debug_assertions)]
            {
                // Create a Jiff Timestamp from seconds and nanoseconds
                let jtf = jt.timestamp().as_second();
                let rtc_time = EPOCH_SIGNAL.wait().await;
                info!("ntp: {}", jtf);
                info!("rtc: {}", rtc_time);
            }
        }
        Err(e) => {
            warn!("Failed to get NTP Time!: {:?}", e);
        }
    }
}

async fn get_webpage() {

    // let mut socket = TcpSocket::new(net_stack, &mut rx_buffer, &mut tx_buffer);

    // socket.set_timeout(Some(embassy_time::Duration::from_secs(10)));

    // let remote_endpoint = (Ipv4Addr::new(142, 250, 185, 115), 123);

    // println!("connecting...");
    // let r = socket.connect(remote_endpoint).await;
    // if let Err(e) = r {
    //     println!("connect error: {:?}", e);
    //     continue;
    // }
    // println!("connected!");
    // let mut buf = [0; 1024];
    // loop {
    //     // use embedded_io_async::Write;
    //     let r = socket
    //         .write/*_all*/(b"GET / HTTP/1.0\r\nHost: www.mobile-j.de\r\n\r\n")
    //         .await;
    //     if let Err(e) = r {
    //         println!("write error: {:?}", e);
    //         break;
    //     }
    //     let n = match socket.read(&mut buf).await {
    //         Ok(0) => {
    //             println!("read EOF");
    //             break;
    //         }
    //         Ok(n) => n,
    //         Err(e) => {
    //             println!("read error: {:?}", e);
    //             break;
    //         }
    //     };
    //     println!("{}", core::str::from_utf8(&buf[..n]).unwrap());
    // }
    // Timer::after_millis(3000).await;
}
