use ds3231::DS3231Error;
use embassy_embedded_hal::shared_bus::I2cDeviceError;

#[derive(Debug, thiserror::Error)]
pub enum RtcError {
    #[error("I2c Error: {0}")]
    I2cError(#[from] esp_hal::i2c::master::Error),
    #[error("Error configuring RTC: {0:?}")]
    DS3231Error(DS3231Error<I2cDeviceError<esp_hal::i2c::master::Error>>),
}

impl From<DS3231Error<I2cDeviceError<esp_hal::i2c::master::Error>>> for RtcError {
    fn from(value: DS3231Error<I2cDeviceError<esp_hal::i2c::master::Error>>) -> Self {
        Self::DS3231Error(value)
    }
}
