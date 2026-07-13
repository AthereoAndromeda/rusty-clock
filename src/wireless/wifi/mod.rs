pub mod dns;
pub mod sntp;
pub mod web_server;

use embassy_executor::Spawner;
use embassy_net::{DhcpConfig, StackResources, driver::Driver};
use embassy_time::Timer;
use esp_hal::rng::Rng;
use esp_radio::wifi::WifiController;

use crate::utils::mk_static;

// TIP: Set these in .env if using direnv
const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

/// Initialize Wifi Stack and attempt to connect to a network.
///
/// # Panics
/// Panics to Wifi Controller fails to initialize.
pub(super) fn init(spawner: Spawner, wifi: esp_hal::peripherals::WIFI<'static>) {
    let (wifi_controller, interfaces) = esp_radio::wifi::new(wifi, Default::default())
        .expect("Failed to initialize Wi-Fi controller");

    defmt::debug!(
        "[wifi:connect] Device capabilities: {:?}",
        interfaces.station.capabilities()
    );

    let (net_stack, net_runner) = get_stack(interfaces);

    spawner.spawn(runner_task(net_runner).unwrap());
    spawner.spawn(connect_to_wifi(wifi_controller).unwrap());
    spawner.spawn(sntp::fetch_sntp(net_stack).unwrap());

    web_server::init(spawner, net_stack);
}

// 3 web tasks + 1 sntp + ? + ?
// Currently requires 6 sockets minimum. Picoserve possibly adds 2 sockets?
const MAX_NET_SOCKETS: usize = web_server::WEB_TASK_POOL_SIZE + 3;

fn get_stack(
    wifi_interface: esp_radio::wifi::Interfaces<'_>,
) -> (
    embassy_net::Stack<'static>,
    embassy_net::Runner<'_, esp_radio::wifi::Interface<'_>>,
) {
    #[cfg(debug_assertions)]
    defmt::debug!("Creating Network Stack...");

    let rng = Rng::new();
    let seed = u64::from(rng.random()) << 32 | u64::from(rng.random());
    let embassy_config = embassy_net::Config::dhcpv4(DhcpConfig::default());

    // Init network stack
    embassy_net::new(
        wifi_interface.station,
        embassy_config,
        mk_static!(
            StackResources<MAX_NET_SOCKETS>;
            StackResources::<MAX_NET_SOCKETS>::new()
        ),
        seed,
    )
}

#[embassy_executor::task]
async fn runner_task(
    mut runner: embassy_net::Runner<'static, esp_radio::wifi::Interface<'static>>,
) {
    runner.run().await;
}

#[embassy_executor::task]
async fn connect_to_wifi(mut controller: WifiController<'static>) -> ! {
    loop {
        if controller.is_connected() {
            // wait until we're no longer connected
            match controller.wait_for_disconnect_async().await {
                Ok(sta) => {
                    defmt::info!("Disconnected: {}", sta);
                }
                Err(err) => {
                    defmt::error!("Failed to disconnect");
                    defmt::error!("{}", err);
                }
            };
            Timer::after_millis(5000).await;
        }

        let station_config = esp_radio::wifi::Config::Station(
            esp_radio::wifi::sta::StationConfig::default()
                .with_ssid(SSID)
                .with_password(PASSWORD.into()),
        );

        #[cfg(debug_assertions)]
        {
            let scan_config = esp_radio::wifi::scan::ScanConfig::default().with_max(10);

            let scan_result = controller.scan_async(&scan_config).await.unwrap();
            for ap in scan_result {
                defmt::debug!("{}", ap);
            }
        }

        controller.set_config(&station_config).unwrap();
        defmt::info!("[wifi:connect] Starting wifi and scan");
        match controller.connect_async().await {
            Ok(sta) => {
                defmt::info!("[wifi:connect] Connected to WiFi. {}", sta);
            }
            Err(e) => {
                defmt::warn!("[wifi:connect] Failed to connect!");
                defmt::warn!("[wifi:connect] {}", e);
                Timer::after_millis(5000).await;
            }
        };
    }
}

// #[embassy_executor::task]
// /// Connect to Wi-Fi
// async fn connect_to_wifi(mut controller: WifiController<'static>) -> ! {
//     #[cfg(debug_assertions)]
//     {
//         use defmt::{debug, trace};
//         trace!("[wifi:connect] start connection task");
//         debug!(
//             "[wifi:connect] Device capabilities: {:?}",
//             controller.capabilities()
//         );
//     }

//     loop {
//         match esp_radio::wifi::sta_state() {
//             WifiStaState::Connected => {
//                 // wait until we're no longer connected
//                 controller.wait_for_event(WifiEvent::StaDisconnected).await;
//                 Timer::after_millis(5000).await;
//             }
//             _ => {
//                 info!("[wifi:connect] Wifi Not Connected!");
//             }
//         }

//         if !matches!(controller.is_started(), Ok(true)) {
//             let station_config = ModeConfig::Client(
//                 ClientConfig::default()
//                     .with_ssid(SSID.into())
//                     .with_password(PASSWORD.into()),
//             );

//             controller.set_config(&station_config).unwrap();
//             info!("[wifi:connect] Starting wifi and scan");
//             controller.start_async().await.unwrap();

//             #[cfg(debug_assertions)]
//             defmt::trace!("[wifi:connect] Wifi started! Scanning for available networks...");

//             let scan_config = esp_radio::wifi::ScanConfig::default().with_max(10);

//             #[expect(clippy::used_underscore_binding, reason = "Used for debugging")]
//             let _scan_result = controller
//                 .scan_with_config_async(scan_config)
//                 .await
//                 .unwrap();

//             #[cfg(debug_assertions)]
//             for ap in _scan_result {
//                 defmt::debug!("{}", ap);
//             }
//         }

//         match controller.connect_async().await {
//             Ok(()) => info!("Wifi connected!"),
//             Err(e) => {
//                 warn!("[wifi:connect] Failed to connect to wifi: {}", e);
//                 Timer::after_millis(5000).await;
//             }
//         }
//     }
// }
