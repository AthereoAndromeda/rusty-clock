pub mod error;
pub mod rtc_time;
pub use error::*;
pub mod alarm;
use alarm::*;

use defmt::info;
use ds3231::{
    Alarm1Config, Config, DS3231, InterruptControl, Oscillator, SquareWaveFrequency,
    TimeRepresentation,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::Timer;
use esp_hal::{
    gpio::{Input, InputConfig, Pull},
    peripherals::{self},
};
use static_cell::StaticCell;

use crate::{
    I2cAsync, TIME_SIGNAL,
    buzzer::{BUZZER_SIGNAL, BuzzerState},
    wireless::wifi::routes::{ALARM_REQUEST, ALARM_SIGNAL, SET_ALARM},
};

pub(crate) type RtcDS3231 = DS3231<I2cAsync>;
pub(crate) const RTC_I2C_ADDR: u8 = 0x68;
pub static RTC_DS3231: StaticCell<Mutex<CriticalSectionRawMutex, RtcDS3231>> = StaticCell::new();

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
    // NOTE: Time stored in RTC is in UTC, adjust to your timezone
    let alarm1_config = if cfg!(debug_assertions) {
        Alarm1Config::AtSeconds { seconds: 30 }
    } else {
        let hours = option_env!("ALARM_HOUR")
            .unwrap_or("0")
            .parse::<u8>()
            .unwrap();
        let minutes = option_env!("ALARM_MINUTES")
            .unwrap_or("0")
            .parse::<u8>()
            .unwrap();
        let seconds = option_env!("ALARM_SECONDS")
            .unwrap_or("0")
            .parse::<u8>()
            .unwrap();

        Alarm1Config::AtTime {
            hours,
            minutes,
            seconds,
            is_pm: None,
        }
    };

    rtc.set_alarm1(&alarm1_config).await?;

    reset_alarm_flags(&mut rtc).await?;

    Ok(rtc)
}

#[embassy_executor::task]
/// Runner for DS3231
///
/// Keeps the time
pub async fn run(rtc_mutex: &'static Mutex<CriticalSectionRawMutex, RtcDS3231>) {
    loop {
        let (datetime, alarm) = {
            let mut rtc = rtc_mutex.lock().await;
            let datetime = rtc.datetime().await.unwrap();
            let alarm = rtc.alarm1().await.unwrap();

            (datetime, alarm)
        };

        TIME_SIGNAL.signal(datetime.into());

        // Listen for alarm requests by web server or GATT
        if ALARM_REQUEST.signaled() {
            ALARM_SIGNAL.signal(alarm);
        }

        if SET_ALARM.signaled() {
            let config = SET_ALARM.try_take().expect("Already waited signal");
            let mut rtc = rtc_mutex.lock().await;
            rtc.set_alarm1(&config).await.unwrap();
            reset_alarm_flags(&mut rtc).await.unwrap();

            SET_ALARM.reset();
        }

        #[cfg(debug_assertions)]
        {
            use crate::TZ_OFFSET;
            use jiff::tz::{Offset, TimeZone};

            let ts = datetime.and_utc().timestamp();
            let ts = jiff::Timestamp::from_second(ts).unwrap();
            let offset = *TZ_OFFSET.get();
            let datetime = ts.to_zoned(TimeZone::fixed(Offset::constant(offset)));

            defmt::info!(
                "{}-{}-{} | {:02}:{:02}:{:02} ({:02}:00)",
                datetime.year(),
                datetime.month(),
                datetime.day(),
                datetime.hour(),
                datetime.minute(),
                datetime.second(),
                offset
            );
        }

        Timer::after_secs(1).await;
    }
}

// TODO: Move to GPIO mod
#[embassy_executor::task]
pub async fn listen_for_alarm(alarm_pin: peripherals::GPIO6<'static>) {
    info!("Initializing Alarm Listener...");
    let mut alarm_input = Input::new(alarm_pin, InputConfig::default().with_pull(Pull::Up));

    // Some time to initialize
    Timer::after_millis(500).await;

    // Beep 3 times
    for _ in 0..3 {
        Timer::after_millis(300).await;
        // buzzer_output.lock().await.toggle();
        BUZZER_SIGNAL.signal(BuzzerState::Toggle);
    }

    // buzzer_output.lock().await.set_low();
    BUZZER_SIGNAL.signal(BuzzerState::Off);

    loop {
        info!("Waiting for alarm...");
        alarm_input.wait_for_falling_edge().await;

        info!("DS3231 Interrupt Received!");
        // buzzer_output.lock().await.set_high();
        BUZZER_SIGNAL.signal(BuzzerState::On);

        #[cfg(debug_assertions)]
        {
            // Stop it from bleeding my ears while devving
            Timer::after_secs(5).await;
            // buzzer_output.lock().await.set_low();
            BUZZER_SIGNAL.signal(BuzzerState::Off);
            info!("Buzzer set low");
        }
    }
}
