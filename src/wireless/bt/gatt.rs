use bt_hci::uuid::{characteristic, descriptors, service};
use trouble_host::prelude::{FromGatt, gatt_server, gatt_service};

#[gatt_server]
/// Our GATT Server
pub struct Server {
    pub battery_service: BatteryService,
    pub time_service: TimeService,
    pub buzzer_service: BuzzerService,
}

/// Battery service
#[gatt_service(uuid = service::BATTERY)]
pub struct BatteryService {
    #[descriptor(uuid = descriptors::VALID_RANGE, read, value = [0, 100])]
    #[descriptor(uuid = descriptors::MEASUREMENT_DESCRIPTION, name = "hello", read, value = "Battery Level")]
    #[characteristic(uuid = characteristic::BATTERY_LEVEL, read, notify, value = 10)]
    /// Battery Level
    pub level: u8,

    #[characteristic(uuid = "408813df-5dd4-1f87-ec11-cdb001100000", write, read, notify)]
    pub status: bool,
}

/// Time Service
#[gatt_service(uuid = service::DEVICE_TIME)]
pub struct TimeService {
    #[descriptor(uuid = descriptors::VALID_RANGE, read, value = [0, 100])]
    #[descriptor(uuid = descriptors::MEASUREMENT_DESCRIPTION, name = "epoch", read, value = "epoch")]
    #[characteristic(uuid = characteristic::DEVICE_TIME, read, notify, value = 10)]
    pub epoch: i64,

    #[characteristic(uuid = "308813df-5dd4-1f87-ec11-cdb001100000", write, read, notify)]
    pub status: bool,
}

/// Buzzer Service
#[gatt_service(uuid = service::COMMON_AUDIO)]
pub struct BuzzerService {
    #[characteristic(uuid = characteristic::AUDIO_OUTPUT_DESCRIPTION, read, write, notify)]
    pub level: bool,

    #[characteristic(uuid = "508813df-5dd4-1f87-ec11-cdb001100000", write, read)]
    pub status: bool,
}
