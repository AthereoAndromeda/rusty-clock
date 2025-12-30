use chrono::{Datelike, Timelike};
use defmt::{error, info};
use ds3231::{DS3231, DS3231Error};
use embassy_time::Timer;
use esp_hal::gpio::{Input, Output};

use crate::{I2cAsync, RTC, RTC_I2C_ADDR};

#[derive(Debug, thiserror::Error)]
pub(crate) enum RtcError {
    #[error("I2c Error: {0}")]
    I2cError(#[from] esp_hal::i2c::master::Error),
    #[error("Error configuring RTC: {0:?}")]
    DS3231Error(DS3231Error<esp_hal::i2c::master::Error>),
}

impl From<DS3231Error<esp_hal::i2c::master::Error>> for RtcError {
    fn from(value: DS3231Error<esp_hal::i2c::master::Error>) -> Self {
        Self::DS3231Error(value)
    }
}

/// Initialize the DS3231 Instance and return RTC
pub async fn init_rtc(i2c: I2cAsync) -> Result<RTC, RtcError> {
    use ds3231::{
        Alarm1Config, Config, DS3231, InterruptControl, Oscillator, SquareWaveFrequency,
        TimeRepresentation,
    };

    let config = Config {
        time_representation: TimeRepresentation::TwentyFourHour,
        square_wave_frequency: SquareWaveFrequency::Hz1,
        interrupt_control: InterruptControl::Interrupt,
        battery_backed_square_wave: false,
        oscillator_enable: Oscillator::Enabled,
    };

    let mut rtc = DS3231::new(i2c, RTC_I2C_ADDR);
    rtc.configure(&config).await?;

    // Hardcoded values for now
    let alarm1_config = Alarm1Config::AtTime {
        hours: 8,
        minutes: 0,
        seconds: 0,
        is_pm: None,
    };

    rtc.set_alarm1(&alarm1_config).await?;

    // Clear any existing alarm flags
    match rtc.status().await {
        Ok(mut status) => {
            status.set_alarm1_flag(false);
            status.set_alarm2_flag(false);
            match rtc.set_status(status).await {
                Ok(_) => info!("Alarm flags cleared"),
                Err(_) => error!("Failed to clear alarm flags"),
            }
        }
        Err(_) => error!("Failed to read status"),
    }

    // Enable Alarm 1 interrupt
    match rtc.control().await {
        Ok(mut control) => {
            control.set_alarm1_interrupt_enable(true);
            control.set_alarm2_interrupt_enable(false);
            match rtc.set_control(control).await {
                Ok(_) => info!("Alarm 1 interrupt enabled"),
                Err(_) => error!("Failed to enable alarm interrupt"),
            }
        }
        Err(_) => error!("Failed to read control register"),
    }

    info!("Starting time monitoring...");
    info!("Current time will be displayed every 100ms when it changes");
    info!("Alarm status will be shown alongside the time");
    info!("SQW/INT pin level will also be monitored");

    Ok(rtc)
}

#[embassy_executor::task]
/// Gets the time and prints every second
pub async fn get_time(mut rtc: DS3231<I2cAsync>) {
    loop {
        let datetime = rtc.datetime().await.unwrap();
        let date = datetime.date();
        let (year, month, day) = (date.year(), date.month(), date.day());

        let time = datetime.time();
        let (hour, minute, second) = (time.hour(), time.minute(), time.second());
        defmt::info!(
            "{}-{}-{} | {:02}:{:02}:{:02}",
            year,
            month,
            day,
            hour,
            minute,
            second
        );
        Timer::after_secs(1).await;
    }
}

#[embassy_executor::task]
pub async fn listen_for_alarm(mut buzzer_output: Output<'static>, mut alarm_input: Input<'static>) {
    defmt::info!("Waiting for alarm...");
    alarm_input.wait_for_falling_edge().await;
    defmt::info!("Received!");

    buzzer_output.set_high();
}
