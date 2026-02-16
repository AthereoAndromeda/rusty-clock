pub mod sntp;
pub mod web_server;

use defmt::{debug, info, warn};
use embassy_executor::Spawner;
use embassy_net::StackResources;
use embassy_time::Timer;
use esp_hal::rng::Rng;
use esp_radio::wifi::{
    ClientConfig, ModeConfig, WifiController, WifiDevice, WifiEvent, WifiStaState,
};

use crate::{PASSWORD, SSID, utils::mk_static};

/// Initialize WiFi Stack and attempt to connect to a network
///
/// # Panics
/// Panics to WiFi Controller fails to initialize
pub(super) fn init(
    spawner: Spawner,
    radio_init: &'static esp_radio::Controller<'static>,
    wifi: esp_hal::peripherals::WIFI<'static>,
) {
    let (wifi_controller, interfaces) = esp_radio::wifi::new(radio_init, wifi, Default::default())
        .expect("Failed to initialize Wi-Fi controller");

    let (net_stack, net_runner) = get_stack(interfaces);

    spawner.must_spawn(runner_task(net_runner));
    spawner.must_spawn(connect_to_wifi(wifi_controller));

    spawner.must_spawn(sntp::fetch_sntp(net_stack));

    let (app, conf) = web_server::init();
    for task_id in 0..web_server::WEB_TASK_POOL_SIZE {
        spawner.must_spawn(web_server::web_task(task_id, net_stack, app, conf));
    }
}

fn get_stack(
    wifi_interface: esp_radio::wifi::Interfaces<'_>,
) -> (
    embassy_net::Stack<'static>,
    embassy_net::Runner<'_, WifiDevice<'_>>,
) {
    info!("Creating Network Stack...");
    let wifi_interface_station = wifi_interface.sta;

    let rng = Rng::new();
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    let embassy_config = embassy_net::Config::dhcpv4(Default::default());

    // 3 web tasks + 1 sntp + ? + ?
    // Currently requires 6 sockets minimum. Picoserve possibly adds 2 sockets?
    const MAX_NET_SOCKETS: usize = web_server::WEB_TASK_POOL_SIZE + 3;

    // Init network stack
    embassy_net::new(
        wifi_interface_station,
        embassy_config,
        mk_static!(
            StackResources<MAX_NET_SOCKETS>,
            StackResources::<MAX_NET_SOCKETS>::new()
        ),
        seed,
    )
}

#[embassy_executor::task]
async fn runner_task(mut runner: embassy_net::Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}

#[embassy_executor::task]
/// Connect to Wi-Fi
async fn connect_to_wifi(mut controller: WifiController<'static>) {
    debug!("start connection task");
    debug!("Device capabilities: {:?}", controller.capabilities());

    loop {
        match esp_radio::wifi::sta_state() {
            WifiStaState::Connected => {
                // wait until we're no longer connected
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                Timer::after_millis(5000).await
            }
            _ => {
                info!("Wifi Not Connected!");
            }
        }

        if !matches!(controller.is_started(), Ok(true)) {
            let station_config = ModeConfig::Client(
                ClientConfig::default()
                    .with_ssid(SSID.into())
                    .with_password(PASSWORD.into()),
            );

            controller.set_config(&station_config).unwrap();
            info!("Starting wifi");
            controller.start_async().await.unwrap();
            info!("Wifi started! Scanning for available networks...");

            let scan_config = esp_radio::wifi::ScanConfig::default().with_max(10);

            let _result = controller
                .scan_with_config_async(scan_config)
                .await
                .unwrap();

            #[cfg(debug_assertions)]
            for ap in _result {
                debug!("{:?}", ap);
            }
        }

        match controller.connect_async().await {
            Ok(_) => info!("Wifi connected!"),
            Err(e) => {
                warn!("Failed to connect to wifi: {:?}", e);
                Timer::after_millis(5000).await
            }
        }
    }
}

// #[embassy_executor::task]
// async fn get_webpage() {

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
// }
