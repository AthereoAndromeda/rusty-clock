pub mod ble_bas_peripheral;
mod gatt;
use ble_bas_peripheral::run_peripheral;

// pub mod ble_bas_central;
// pub mod ble_l2cap_peripheral;
// pub mod ble_time_peripheral;

use bt_hci::{controller::ExternalController, uuid::appearance};
use defmt::info;
use embassy_executor::Spawner;
use esp_radio::ble::controller::BleConnector;
use trouble_host::{
    Address, HostResources,
    gap::{GapConfig, PeripheralConfig},
    prelude::DefaultPacketPool,
};

use crate::{MAC_ADDR, utils::mk_static};

/// Max number of connections for Bluetooth
const BLE_CONNECTIONS_MAX: usize = 2;

/// Max number of L2CAP channels.
const L2CAP_CHANNELS_MAX: usize = 3; // Signal + att + CoC
const BLE_SLOTS: usize = 8;
type BleController = ExternalController<BleConnector<'static>, BLE_SLOTS>;
type BleResources = HostResources<DefaultPacketPool, BLE_CONNECTIONS_MAX, L2CAP_CHANNELS_MAX>;
type BleStack = trouble_host::Stack<
    'static,
    ExternalController<BleConnector<'static>, BLE_SLOTS>,
    DefaultPacketPool,
>;

pub(super) fn init(
    spawner: Spawner,
    radio_init: &'static esp_radio::Controller<'static>,
    bt: esp_hal::peripherals::BT<'static>,
) {
    // find more examples https://github.com/embassy-rs/trouble/tree/main/examples/esp32
    let transport = BleConnector::new(radio_init, bt, Default::default()).unwrap();
    let ble_controller: BleController = ExternalController::new(transport);

    let (ble_stack, ble_host, gatt_server) = get_stack(ble_controller);
    spawner.must_spawn(runner_task(ble_host.runner));
    spawner.must_spawn(run_peripheral(ble_host.peripheral, gatt_server, ble_stack));
}

#[embassy_executor::task]
/// Background dunner for bluetooth
///
/// # Warning
/// Must be ran in the background for BLE to work!
async fn runner_task(
    mut runner: trouble_host::prelude::Runner<'static, BleController, DefaultPacketPool>,
) {
    runner.run().await.unwrap();
}

fn get_stack(
    ble_controller: BleController,
) -> (
    &'static BleStack,
    trouble_host::Host<'static, BleController, DefaultPacketPool>,
    gatt::Server<'static>,
) {
    // Using a fixed "random" address can be useful for testing. In real scenarios, one would
    // use e.g. the MAC 6 byte array as the address (how to get that varies by the platform).
    let address: Address = Address::random(MAC_ADDR);
    info!("Our address = {}", address.addr);

    let ble_resources = mk_static!(BleResources, HostResources::new());
    let ble_stack: &'static BleStack = mk_static!(
        BleStack,
        trouble_host::new(ble_controller, ble_resources).set_random_address(address)
    );

    let ble_host = ble_stack.build();

    let gatt_server = gatt::Server::new_with_config(GapConfig::Peripheral(PeripheralConfig {
        name: "TrouBLE",
        appearance: &appearance::CLOCK,
    }))
    .unwrap();

    (ble_stack, ble_host, gatt_server)
}

// PSM from the dynamic range (0x0080-0x00FF) according to the Bluetooth
// Specification for L2CAP channels using LE Credit Based Flow Control mode.
// used for the BLE L2CAP examples.
//
// https://www.bluetooth.com/wp-content/uploads/Files/Specification/HTML/Core-60/out/en/host/logical-link-control-and-adaptation-protocol-specification.html#UUID-1ffdf913-7b8a-c7ba-531e-2a9c6f6da8fb
//
// pub(crate) const PSM_L2CAP_EXAMPLES: u16 = 0x0081;
