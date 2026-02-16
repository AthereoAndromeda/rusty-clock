use embassy_executor::Spawner;
use esp_hal::peripherals;

use crate::utils::mk_static;

#[cfg(feature = "ble")]
mod bt;
mod wifi;

/// Initialize the esp-radio Controller and
/// WiFi/BLE functionality
///
/// # Panics
/// Panics if [`esp_radio::init()`] fails to initialize the controller
pub(crate) fn init(
    spawner: Spawner,
    wifi: peripherals::WIFI<'static>,
    #[cfg(feature = "ble")] bt: peripherals::BT<'static>,
) {
    let radio_init: &'static esp_radio::Controller<'static> = mk_static!(
        esp_radio::Controller<'static>;
        esp_radio::init().expect("Failed to init WiFi/BLE Controller")
    );

    wifi::init(spawner, radio_init, wifi);

    #[cfg(feature = "ble")]
    bt::init(spawner, radio_init, bt);
}
