use super::gatt::Server;
use chrono::Timelike;
use defmt::{error, info, warn};
use embassy_futures::select::select;
use embassy_time::Timer;
use trouble_host::prelude::*;

use super::{BleController, BleStack};
use crate::{buzzer::BUZZER_ACTION_SIGNAL, rtc_ds3231::TIME_WATCH, utils::mk_static};

#[embassy_executor::task]
pub(super) async fn run_peripheral(
    mut peripheral: Peripheral<'static, BleController, DefaultPacketPool>,
    server: Server<'static>,
    stack: &'static BleStack,
) {
    info!("Starting advertising and GATT service");
    let server: &'static Server<'static> = mk_static!(Server<'static>, server);

    loop {
        match advertise("Rusty Alarm Clock", &mut peripheral, server).await {
            Ok(conn) => {
                gatt_events_task(server, &conn, stack).await;
                // spawner.must_spawn(battery_task(&server, &conn, &stack));
                // spawner.must_spawn(time_task(&server, &conn));
            }
            Err(e) => {
                let e = defmt::Debug2Format(&e);
                panic!("[adv] error: {:?}", e);
            }
        }
    }
}

macro_rules! server_get {
    ($($id:ident),*;$ev:ident, $srv:ident) => {{
        $(
            if $ev.handle() == $id.handle {
                let value = $srv.get(&$id).expect("Failed to get characteristic");
                info!("[gatt] Read Event to {} Characteristic: {}", stringify!($id), value);
            }
        )*
    }};
}

/// Stream Events until the connection closes.
///
/// This function will handle the GATT events and process them.
/// This is how we interact with read and write requests.
async fn gatt_events_task(
    server: &'static Server<'static>,
    conn: &GattConnection<'_, '_, DefaultPacketPool>,
    _stack: &'static BleStack,
) {
    let level = server.battery_service.level;
    let time_epoch_char = server.time_service.epoch;
    let buzzer = server.buzzer_service.level;

    let mut recv = TIME_WATCH
        .receiver()
        .expect("Maximum Number of receivers reached");

    loop {
        let a = async {
            let time = recv.get().await;
            let sec = time.second() as i64;

            info!("[time_task] notifying connection of time {}", time);
            if time_epoch_char.notify(conn, &sec).await.is_err() {
                error!("[time_task] error notifying connection");
            };
        };

        let b = async {
            match conn.next().await {
                GattConnectionEvent::Disconnected { reason } => {
                    info!("Disconnected: {}", reason);
                }
                GattConnectionEvent::Gatt { event } => {
                    match &event {
                        GattEvent::Read(event) => {
                            server_get!(level, time_epoch_char, buzzer; event, server);
                        }
                        GattEvent::Write(event) => {
                            // server_write!(level, epoch, buzzer; event, server);
                            if event.handle() == buzzer.handle {
                                let w = server.get(&buzzer).unwrap();
                                BUZZER_ACTION_SIGNAL.signal(w.into());
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

        select(a, b).await;
        Timer::after_millis(100).await;
    }

    // info!("[gatt] disconnected: {:?}", reason);
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

// #[embassy_executor::task]
// /// Example task to use the BLE notifier interface.
// /// This task will notify the connected central of a counter value every 2 seconds.
// /// It will also read the RSSI value every 2 seconds.
// /// and will stop when the connection is closed by the central or an error occurs.
// async fn battery_task(
//     server: &'static Server<'static>,
//     conn: &'static GattConnection<'static, 'static, DefaultPacketPool>,
//     stack: &'static BleStack,
// ) {
//     let mut tick: u8 = 0;
//     let level = server.battery_service.level;

//     loop {
//         tick = tick.wrapping_add(1);
//         // info!("Tick is: {}", tick);
//         info!("[custom_task] notifying connection of tick {}", tick);
//         if level.notify(conn, &tick).await.is_err() {
//             error!("[custom_task] error notifying connection");
//             break;
//         };

//         // read RSSI (Received Signal Strength Indicator) of the connection.
//         if let Ok(rssi) = conn.raw().rssi(stack).await {
//             info!("[custom_task] RSSI: {:?}", rssi);
//         } else {
//             error!("[custom_task] error getting RSSI");
//             break;
//         };
//         Timer::after_secs(2).await;
//     }
// }
