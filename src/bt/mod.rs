pub mod ble_bas_peripheral;
pub use ble_bas_peripheral::run_peripheral;
// pub mod ble_bas_central;
// pub mod ble_l2cap_peripheral;
// pub mod ble_time_peripheral;

// PSM from the dynamic range (0x0080-0x00FF) according to the Bluetooth
// Specification for L2CAP channels using LE Credit Based Flow Control mode.
// used for the BLE L2CAP examples.
//
// https://www.bluetooth.com/wp-content/uploads/Files/Specification/HTML/Core-60/out/en/host/logical-link-control-and-adaptation-protocol-specification.html#UUID-1ffdf913-7b8a-c7ba-531e-2a9c6f6da8fb
//
// pub(crate) const PSM_L2CAP_EXAMPLES: u16 = 0x0081;

// /// Max number of connections
// pub(crate) const CONNECTIONS_MAX: usize = 1;

// /// Max number of L2CAP channels.
// pub(crate) const L2CAP_CHANNELS_MAX: usize = 2; // Signalunwrap_or_else
