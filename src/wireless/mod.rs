use bt_hci::controller::ExternalController;
use esp_hal::peripherals;
use esp_radio::{
    ble::controller::BleConnector,
    wifi::{Interfaces, WifiController},
};

use crate::{bt::BleController, wireless::bt::ble_bas_peripheral::RADIO_INIT};

pub mod bt;
pub mod wifi;

pub fn init_wireless(
    wifi: peripherals::WIFI<'static>,
    bt: peripherals::BT<'static>,
) -> (WifiController<'static>, Interfaces<'static>, BleController) {
    let radio_init: &'static mut esp_radio::Controller<'static> =
        RADIO_INIT.init(esp_radio::init().expect("Failed to init Wifi/BLE controller"));

    let (wifi_controller, interfaces) = esp_radio::wifi::new(radio_init, wifi, Default::default())
        .expect("Failed to initialize Wi-Fi controller");

    // find more examples https://github.com/embassy-rs/trouble/tree/main/examples/esp32
    let transport = BleConnector::new(radio_init, bt, Default::default()).unwrap();
    let ble_controller: BleController = ExternalController::new(transport);

    (wifi_controller, interfaces, ble_controller)
}
