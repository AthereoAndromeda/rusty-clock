pub mod sntp;
pub mod web_server;

pub const MAX_NET_SOCKETS: usize = 6;

use defmt::{debug, info, warn};
use embassy_net::StackResources;
use embassy_time::Timer;
use esp_hal::rng::Rng;
use esp_radio::wifi::{
    ClientConfig, ModeConfig, WifiController, WifiDevice, WifiEvent, WifiStaState,
};

use crate::{PASSWORD, SSID, mk_static};

pub fn get_net_stack(
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
pub async fn net_runner_task(mut runner: embassy_net::Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}

#[embassy_executor::task]
/// Connect to Wi-Fi
pub async fn connect_to_wifi(mut controller: WifiController<'static>) {
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

            #[allow(unused_assignments)]
            let result = controller
                .scan_with_config_async(scan_config)
                .await
                .unwrap();

            #[cfg(debug_assertions)]
            for ap in result {
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
