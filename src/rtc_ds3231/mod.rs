pub mod rtc_time;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
pub use rtc_time::*;

use defmt::{info, warn};
use ds3231::{
    Alarm1Config, Config, DS3231, DS3231Error, InterruptControl, Oscillator, SquareWaveFrequency,
    TimeRepresentation,
};
use embassy_time::{Duration, Timer, WithTimeout};
use esp_hal::{
    gpio::{DriveStrength, Input, InputConfig, Level, Output, OutputConfig, Pull},
    peripherals,
};

use crate::{EPOCH_SIGNAL, I2cAsync, NTP_SIGNAL, TIME_SIGNAL};

pub(crate) type RtcDS3231 = DS3231<I2cAsync>;
pub(crate) const RTC_I2C_ADDR: u8 = 0x68;

type EspHalI2cErr = esp_hal::i2c::master::Error;

#[derive(Debug, thiserror::Error)]
pub(crate) enum RtcError {
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

/// Initialize the DS3231 Instance and return RTC
pub async fn init_rtc(i2c: I2cAsync) -> Result<RtcDS3231, RtcError> {
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
    let alarm1_config = if cfg!(debug_assertions) {
        Alarm1Config::AtSeconds { seconds: 30 }
    } else {
        Alarm1Config::AtTime {
            hours: 8,
            minutes: 0,
            seconds: 0,
            is_pm: None,
        }
    };

    rtc.set_alarm1(&alarm1_config).await?;

    let mut status = rtc.status().await?;
    status.set_alarm1_flag(false);
    status.set_alarm2_flag(false);
    rtc.set_status(status).await?;

    #[cfg(debug_assertions)]
    info!("Alarm flags cleared");

    // Enable Alarm 1 interrupt
    let mut control = rtc.control().await?;
    control.set_alarm1_interrupt_enable(true);
    control.set_alarm2_interrupt_enable(false);
    rtc.set_control(control).await?;

    #[cfg(debug_assertions)]
    info!("Alarm 1 interrupt enabled");

    // #[cfg(debug_assertions)]
    // {
    //     info!("Starting time monitoring...");
    //     info!("Current time will be displayed every 100ms when it changes");
    //     info!("Alarm status will be shown alongside the time");
    //     info!("SQW/INT pin level will also be monitored");
    // }

    Ok(rtc)
}

// #[embassy_executor::task]
// /// Gets the time and prints every second
// pub async fn run(rtc: &'static mut RtcDS3231) {
//     let ntp_value = NTP_SIGNAL.wait();
//     let timeout = Timer::after_secs(60 * 10);

//     loop {
//         let datetime = rtc.datetime().await.unwrap();
//         TIME_SIGNAL.signal(datetime.into());
//         EPOCH_SIGNAL.signal(datetime.and_utc().timestamp());

//         let datetime: RtcTime = datetime.into();
//         #[cfg(debug_assertions)]
//         defmt::info!(
//             "{}-{}-{} | {:02}:{:02}:{:02}",
//             datetime.year,
//             datetime.month,
//             datetime.day,
//             datetime.hour,
//             datetime.minute,
//             datetime.second
//         );

//         select()

//         Timer::after_secs(1).await;
//     }
// }

// use embassy_futures::select::{Either3, select3};

// #[embassy_executor::task]
// pub async fn run(rtc: &'static mut RtcDS3231) {
//     let mut ntp_fut = NTP_SIGNAL.wait();
//     let mut timeout_fut = Timer::after_secs(60 * 10);

//     loop {
//         let tick_fut = Timer::after_secs(1);

//         match select3(&mut ntp_fut, &mut timeout_fut, tick_fut).await {
//             Either3::First(ntp_val) => {
//                 info!("NTP Signal Received!");
//                 let datetime = chrono::DateTime::from_timestamp_secs(ntp_val)
//                     .unwrap()
//                     .naive_utc();

//                 rtc.set_datetime(&datetime).await.unwrap();
//                 info!("RTC Value changed");
//             }
//             // Case 2: 10 minute timeout reached!
//             Either3::Second(_) => {
//                 defmt::warn!("NTP sync timed out after 10 minutes!");
//             }
//             // Case 3: The 1-second tick happened (Normal Loop)
//             Either3::Third(_) => {
//                 let datetime = rtc.datetime().await.unwrap();

//                 // Update signals
//                 TIME_SIGNAL.signal(datetime.into());
//                 EPOCH_SIGNAL.signal(datetime.and_utc().timestamp());

//                 #[cfg(debug_assertions)]
//                 {
//                     use jiff::tz::{Offset, TimeZone};

//                     // let datetime: RtcTime = datetime.and_local_timezone().into();
//                     let datetime = datetime;

//                     let ts = jiff::Timestamp::from_second(datetime.and_utc().timestamp()).unwrap();
//                     let datetime = ts.to_zoned(TimeZone::fixed(Offset::from_hours(8).unwrap()));

//                     defmt::info!(
//                         "{}-{}-{} | {:02}:{:02}:{:02}",
//                         datetime.year(),
//                         datetime.month(),
//                         datetime.day(),
//                         datetime.hour(),
//                         datetime.minute(),
//                         datetime.second()
//                     );
//                 }
//             }
//         }
//     }
// }

#[embassy_executor::task]
pub async fn run(rtc_mutex: &'static Mutex<CriticalSectionRawMutex, RtcDS3231>) {
    loop {
        let datetime = rtc_mutex.lock().await.datetime().await.unwrap();

        TIME_SIGNAL.signal(datetime.into());
        EPOCH_SIGNAL.signal(datetime.and_utc().timestamp());

        let datetime: RtcTime = datetime.into();

        #[cfg(debug_assertions)]
        defmt::info!(
            "{}-{}-{} | {:02}:{:02}:{:02}",
            datetime.year,
            datetime.month,
            datetime.day,
            datetime.hour,
            datetime.minute,
            datetime.second
        );

        Timer::after_secs(1).await;
    }
}

#[embassy_executor::task]

pub async fn update_rtc(rtc_mutex: &'static Mutex<CriticalSectionRawMutex, RtcDS3231>) {
    let signal = NTP_SIGNAL
        .wait()
        .with_timeout(Duration::from_secs(60 * 3))
        .await;

    match signal {
        Ok(ntp) => {
            info!("Setting RTC Datetime to NTP...");
            let datetime = chrono::DateTime::from_timestamp_secs(ntp)
                .unwrap()
                .naive_utc();
            let mut rtc = rtc_mutex.lock().await;

            rtc.set_datetime(&datetime).await.unwrap();
            info!("Succesfully Set RTC Datetime!");
        }
        Err(e) => {
            warn!("Failed to get NTP Service: {:?}", e);
        }
    }
}

#[embassy_executor::task]
pub async fn listen_for_alarm(
    output_pin: peripherals::GPIO5<'static>,
    alarm_pin: peripherals::GPIO6<'static>,
) {
    info!("Initializing Alarm Listener...");
    let mut alarm_input = Input::new(alarm_pin, InputConfig::default().with_pull(Pull::Up));

    let mut buzzer_output = Output::new(
        output_pin,
        Level::High,
        // Possibly changw to Pull::Down to remove need for resistor
        OutputConfig::default().with_drive_strength(DriveStrength::_5mA),
    );

    // Some time to initialize
    Timer::after_millis(500).await;

    // Beep 3 times
    for _ in 0..3 {
        esp_hal::delay::Delay::new().delay_millis(300);
        buzzer_output.toggle();
    }

    buzzer_output.set_low();

    info!("Waiting for alarm...");
    alarm_input.wait_for_falling_edge().await;

    info!("DS3231 Interrupt Received!");
    buzzer_output.set_high();

    #[cfg(debug_assertions)]
    {
        // Stop it from bleeding my ears while devving
        Timer::after_secs(10).await;
        buzzer_output.set_low();
        info!("Buzzer set low");
    }
}
