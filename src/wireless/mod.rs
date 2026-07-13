use embassy_executor::Spawner;
use esp_hal::peripherals;

#[cfg(feature = "ble")]
mod bt;
mod wifi;

/// Initialize the esp-radio Controller and
/// WiFi/BLE functionality.
///
/// # Panics
/// Panics if [`esp_radio::init()`] fails to initialize the controller.
pub(crate) fn init(
    spawner: Spawner,
    wifi: peripherals::WIFI<'static>,
    #[cfg(feature = "ble")] bt: peripherals::BT<'static>,
) {
    wifi::init(spawner, wifi);

    #[cfg(feature = "ble")]
    bt::init(spawner, radio_init, bt);
}
