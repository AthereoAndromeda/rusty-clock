pub mod ble_bas_peripheral;
pub use ble_bas_peripheral::run_peripheral;

// pub mod ble_bas_central;
// pub mod ble_l2cap_peripheral;
// pub mod ble_time_peripheral;

use bt_hci::controller::ExternalController;
use esp_radio::ble::controller::BleConnector;
use trouble_host::{HostResources, prelude::DefaultPacketPool};

/// Max number of connections for Bluetooth
pub const BLE_CONNECTIONS_MAX: usize = 2;

/// Max number of L2CAP channels.
pub const L2CAP_CHANNELS_MAX: usize = 3; // Signal + att + CoC
pub const BLE_SLOTS: usize = 8;
pub type BleController = ExternalController<BleConnector<'static>, BLE_SLOTS>;
pub type BleResources = HostResources<DefaultPacketPool, BLE_CONNECTIONS_MAX, L2CAP_CHANNELS_MAX>;
pub type BleStack = trouble_host::Stack<
    'static,
    ExternalController<BleConnector<'static>, BLE_SLOTS>,
    DefaultPacketPool,
>;

// PSM from the dynamic range (0x0080-0x00FF) according to the Bluetooth
// Specification for L2CAP channels using LE Credit Based Flow Control mode.
// used for the BLE L2CAP examples.
//
// https://www.bluetooth.com/wp-content/uploads/Files/Specification/HTML/Core-60/out/en/host/logical-link-control-and-adaptation-protocol-specification.html#UUID-1ffdf913-7b8a-c7ba-531e-2a9c6f6da8fb
//
// pub(crate) const PSM_L2CAP_EXAMPLES: u16 = 0x0081;
