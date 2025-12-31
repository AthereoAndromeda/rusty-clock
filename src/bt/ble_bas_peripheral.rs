use defmt::{error, info, warn};
use embassy_futures::{join, select::select3};
use embassy_time::Timer;
use esp_hal::peripherals;
use esp_radio::{
    ble::controller::BleConnector,
    wifi::{Interfaces, WifiController},
};
use static_cell::StaticCell;
use trouble_host::prelude::*;

pub static RADIO_INIT: StaticCell<esp_radio::Controller<'static>> = StaticCell::new();

use crate::{BleController, BleStack, EPOCH_SIGNAL, TIME_SIGNAL};

/// Must be ran before ble_tasks
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
    let ble_controller: BleController = ExternalController::<_, 20>::new(transport);

    (wifi_controller, interfaces, ble_controller)
}

#[embassy_executor::task]
/// Background dunner for bluetooth
///
/// # Warning
/// Must be ran in the background for BLE to work!
pub async fn ble_runner_task(
    mut runner: trouble_host::prelude::Runner<'static, BleController, DefaultPacketPool>,
) {
    runner.run().await.unwrap();
}

#[gatt_server]
/// Our GATT Server
///
/// It has fields for
/// - Battery
/// - TimeService
pub(crate) struct Server {
    battery_service: BatteryService,
    time_service: TimeService,
}

/// Battery service
#[gatt_service(uuid = service::BATTERY)]
pub(crate) struct BatteryService {
    /// Battery Level
    #[descriptor(uuid = descriptors::VALID_RANGE, read, value = [0, 100])]
    #[descriptor(uuid = descriptors::MEASUREMENT_DESCRIPTION, name = "hello", read, value = "Battery Level")]
    #[characteristic(uuid = characteristic::BATTERY_LEVEL, read, notify, value = 10)]
    level: u8,
    #[characteristic(uuid = "408813df-5dd4-1f87-ec11-cdb001100000", write, read, notify)]
    status: bool,
}

/// Time Service
#[gatt_service(uuid = service::DEVICE_TIME)]
pub(crate) struct TimeService {
    /// Time
    #[descriptor(uuid = descriptors::VALID_RANGE, read, value = [0, 100])]
    #[descriptor(uuid = descriptors::MEASUREMENT_DESCRIPTION, name = "time", read, value = "Time!")]
    #[characteristic(uuid = characteristic::DEVICE_TIME, read, notify, value = 10)]
    level: u8,
    #[characteristic(uuid = "308813df-5dd4-1f87-ec11-cdb001100000", write, read, notify)]
    status: bool,

    #[descriptor(uuid = descriptors::MEASUREMENT_DESCRIPTION, name = "epoch", read, value = "EPOCj!")]
    #[characteristic(uuid = characteristic::DEVICE_TIME, write, read, notify)]
    epoch: i64,
}

#[embassy_executor::task]
pub async fn run_peripheral(
    mut peripheral: Peripheral<'static, BleController, DefaultPacketPool>,
    server: Server<'static>,
    stack: &'static BleStack,
) {
    info!("Starting advertising and GATT service");
    loop {
        match advertise("Trouble Example", &mut peripheral, &server).await {
            Ok(conn) => {
                // set up tasks when the connection is established to a central, so they don't run when no one is connected.
                let a = gatt_events_task(&server, &conn);
                let b = custom_task(&server, &conn, &stack);
                let c = time_task::<BleController, _>(&server, &conn);

                // run until any task ends (usually because the connection has been closed),
                // then return to advertising state.
                // select(a, b).await;
                select3(a, b, c).await;
            }
            Err(e) => {
                let e = defmt::Debug2Format(&e);
                panic!("[adv] error: {:?}", e);
            }
        }
    }
}

/// Stream Events until the connection closes.
///
/// This function will handle the GATT events and process them.
/// This is how we interact with read and write requests.
async fn gatt_events_task<P: PacketPool>(
    server: &Server<'_>,
    conn: &GattConnection<'_, '_, P>,
) -> Result<(), Error> {
    let level = server.battery_service.level;
    let reason = loop {
        match conn.next().await {
            GattConnectionEvent::Disconnected { reason } => break reason,
            GattConnectionEvent::Gatt { event } => {
                match &event {
                    GattEvent::Read(event) => {
                        if event.handle() == level.handle {
                            let value = server.get(&level).expect("Failed to get characteristic");
                            info!("[gatt] Read Event to Level Characteristic: {}", value);
                        }
                    }
                    GattEvent::Write(event) => {
                        if event.handle() == level.handle {
                            info!(
                                "[gatt] Write Event to Level Characteristic: {:?}",
                                event.data()
                            );
                        }
                    }
                    _ => {}
                };
                // This step is also performed at drop(), but writing it explicitly is necessary
                // in order to ensure reply is sent.
                match event.accept() {
                    Ok(reply) => reply.send().await,
                    Err(_e) => warn!("[gatt] error sending response: "),
                };
            }
            _ => {} // ignore other Gatt Connection Events
        }
    };
    info!("[gatt] disconnected: {:?}", reason);
    Ok(())
}

/// Create an advertiser to use to connect to a BLE Central, and wait for it to connect.
async fn advertise<'values, 'server, C: Controller>(
    name: &'values str,
    peripheral: &mut Peripheral<'values, C, DefaultPacketPool>,
    server: &'server Server<'values>,
) -> Result<GattConnection<'values, 'server, DefaultPacketPool>, BleHostError<C::Error>> {
    let mut advertiser_data = [0; 31];
    let len = AdStructure::encode_slice(
        &[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::ServiceUuids16(&[[0x0f, 0x18]]),
            AdStructure::CompleteLocalName(name.as_bytes()),
        ],
        &mut advertiser_data[..],
    )?;
    let advertiser = peripheral
        .advertise(
            &Default::default(),
            Advertisement::ConnectableScannableUndirected {
                adv_data: &advertiser_data[..len],
                scan_data: &[],
            },
        )
        .await?;
    info!("[adv] advertising");
    let conn = advertiser.accept().await?.with_attribute_server(server)?;
    info!("[adv] connection established");
    Ok(conn)
}

/// Example task to use the BLE notifier interface.
/// This task will notify the connected central of a counter value every 2 seconds.
/// It will also read the RSSI value every 2 seconds.
/// and will stop when the connection is closed by the central or an error occurs.
async fn custom_task<C: Controller, P: PacketPool>(
    server: &Server<'_>,
    conn: &GattConnection<'_, '_, P>,
    stack: &Stack<'_, C, P>,
) {
    let mut tick: u8 = 0;
    let level = server.battery_service.level;

    loop {
        tick = tick.wrapping_add(1);
        // info!("Tick is: {}", tick);
        info!("[custom_task] notifying connection of tick {}", tick);
        if level.notify(conn, &tick).await.is_err() {
            error!("[custom_task] error notifying connection");
            break;
        };

        // read RSSI (Received Signal Strength Indicator) of the connection.
        if let Ok(rssi) = conn.raw().rssi(stack).await {
            info!("[custom_task] RSSI: {:?}", rssi);
        } else {
            error!("[custom_task] error getting RSSI");
            break;
        };
        Timer::after_secs(2).await;
    }
}

async fn time_task<C: Controller, P: PacketPool>(
    server: &Server<'_>,
    conn: &GattConnection<'_, '_, P>,
) {
    let time_char = server.time_service.level;
    let epoch_char = server.time_service.epoch;

    loop {
        let time = TIME_SIGNAL.wait().await;
        let epoch = EPOCH_SIGNAL.wait().await;

        let fut1 = async {
            info!("[time_task] notifying connection of time {}", time);
            if time_char.notify(conn, &time.second).await.is_err() {
                error!("[time_task] error notifying connection");
            };
        };

        let fut2 = async {
            info!("[time_task] notifying connection of epoch {}", epoch);
            if epoch_char.notify(conn, &epoch).await.is_err() {
                error!("[time_task] error notifying connection");
            };
        };

        join::join(fut1, fut2).await;
        Timer::after_secs(1).await;
    }
}
