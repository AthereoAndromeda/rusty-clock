use bt_hci::controller::ExternalController;
use embassy_executor::Spawner;
use esp_hal::peripherals;
use esp_radio::ble::controller::BleConnector;

use crate::{
    bt::BleController,
    mk_static,
    rtc_ds3231::RtcMutex,
    wireless::{
        bt::{ble_runner_task, get_ble_stack},
        wifi::{
            connect_to_wifi, get_net_stack, net_runner_task,
            sntp::fetch_sntp,
            web_server::{WEB_TASK_POOL_SIZE, init_web, web_task},
        },
    },
};

pub mod bt;
pub mod wifi;

pub fn init_wireless(
    spawner: Spawner,
    wifi: peripherals::WIFI<'static>,
    bt: peripherals::BT<'static>,
    rtc: &'static RtcMutex, // ) -> (WifiController<'static>, Interfaces<'static>, BleController) {
) {
    // let radio_init: &'static mut esp_radio::Controller<'static> =
    // RADIO_INIT.init(esp_radio::init().expect("Failed to init Wifi/BLE controller"));

    let radio_init: &'static mut esp_radio::Controller<'static> = mk_static!(
        esp_radio::Controller<'static>,
        esp_radio::init().expect("Failed to init WiFi/BLE Controller")
    );

    let (wifi_controller, interfaces) = esp_radio::wifi::new(radio_init, wifi, Default::default())
        .expect("Failed to initialize Wi-Fi controller");

    // find more examples https://github.com/embassy-rs/trouble/tree/main/examples/esp32
    let transport = BleConnector::new(radio_init, bt, Default::default()).unwrap();
    let ble_controller: BleController = ExternalController::new(transport);

    let (net_stack, net_runner) = get_net_stack(interfaces);
    let (ble_stack, ble_host, gatt_server) = get_ble_stack(ble_controller);

    spawner.must_spawn(ble_runner_task(ble_host.runner));
    spawner.must_spawn(net_runner_task(net_runner));
    spawner.must_spawn(connect_to_wifi(wifi_controller));

    spawner.must_spawn(bt::run_peripheral(
        ble_host.peripheral,
        gatt_server,
        ble_stack,
    ));
    spawner.must_spawn(fetch_sntp(net_stack, rtc));

    let (app, conf) = init_web();

    for task_id in 0..WEB_TASK_POOL_SIZE {
        spawner.must_spawn(web_task(task_id, net_stack, app, conf));
    }
    // (wifi_controller, interfaces, ble_controller)
}
