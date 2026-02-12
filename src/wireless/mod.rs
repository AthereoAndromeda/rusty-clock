use bt_hci::controller::ExternalController;
use embassy_executor::Spawner;
use esp_hal::peripherals;
use esp_radio::ble::controller::BleConnector;

use crate::mk_static;

mod bt;
mod wifi;

pub(crate) fn init_wireless(
    spawner: Spawner,
    wifi: peripherals::WIFI<'static>,
    bt: peripherals::BT<'static>,
) {
    let radio_init: &'static mut esp_radio::Controller<'static> = mk_static!(
        esp_radio::Controller<'static>,
        esp_radio::init().expect("Failed to init WiFi/BLE Controller")
    );

    let (wifi_controller, interfaces) = esp_radio::wifi::new(radio_init, wifi, Default::default())
        .expect("Failed to initialize Wi-Fi controller");

    // find more examples https://github.com/embassy-rs/trouble/tree/main/examples/esp32
    let transport = BleConnector::new(radio_init, bt, Default::default()).unwrap();
    let ble_controller: bt::BleController = ExternalController::new(transport);

    let (net_stack, net_runner) = wifi::get_net_stack(interfaces);
    let (ble_stack, ble_host, gatt_server) = bt::get_ble_stack(ble_controller);

    spawner.must_spawn(bt::ble_runner_task(ble_host.runner));
    spawner.must_spawn(wifi::net_runner_task(net_runner));
    spawner.must_spawn(wifi::connect_to_wifi(wifi_controller));
    spawner.must_spawn(bt::run_peripheral(
        ble_host.peripheral,
        gatt_server,
        ble_stack,
    ));
    spawner.must_spawn(wifi::sntp::fetch_sntp(net_stack));

    let (app, conf) = wifi::web_server::init_web();
    for task_id in 0..wifi::web_server::WEB_TASK_POOL_SIZE {
        spawner.must_spawn(wifi::web_server::web_task(task_id, net_stack, app, conf));
    }
}
