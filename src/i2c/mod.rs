use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;

pub type I2cAsync = esp_hal::i2c::master::I2c<'static, esp_hal::Async>;
pub type I2cBus = embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice<
    'static,
    CriticalSectionRawMutex,
    I2cAsync,
>;
