use ds3231::DS3231Error;

type EspHalI2cErr = esp_hal::i2c::master::Error;

#[derive(Debug, thiserror::Error)]
pub enum RtcError {
    #[error("I2c Error: {0}")]
    I2cError(#[from] EspHalI2cErr),
    #[error("Error configuring RTC: {0:?}")]
    DS3231Error(DS3231Error<EspHalI2cErr>),
}

impl From<DS3231Error<esp_hal::i2c::master::Error>> for RtcError {
    fn from(value: DS3231Error<esp_hal::i2c::master::Error>) -> Self {
        Self::DS3231Error(value)
    }
}
