pub mod error;
pub mod rtc_time;
use core::sync::atomic::AtomicBool;

pub use error::*;
pub mod alarm;
use alarm::*;

use defmt::{debug, info};
use ds3231::{
    Alarm1Config, Config, DS3231, InterruptControl, Oscillator, SquareWaveFrequency,
    TimeRepresentation,
};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex, signal::Signal, watch::Watch,
};
use embassy_time::Timer;
use esp_hal::{
    gpio::{Input, InputConfig, Pull},
    peripherals::{self},
};
use static_cell::StaticCell;

use crate::{
    I2cAsync,
    buzzer::{BUZZER_SIGNAL, BuzzerAction},
    rtc_ds3231::rtc_time::RtcTime,
};

pub static TIME_WATCH: Watch<CriticalSectionRawMutex, RtcTime, 5> = Watch::new();
pub static ALARM_SIGNAL: Signal<CriticalSectionRawMutex, Alarm1Config> = Signal::new();
pub static SET_ALARM: Signal<CriticalSectionRawMutex, Alarm1Config> = Signal::new();

pub static IS_ALARM_REQUESTED: AtomicBool = AtomicBool::new(false);

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

    debug!("{:?}", alarm1_config);

    rtc.set_alarm1(&alarm1_config).await?;

    reset_alarm_flags(&mut rtc).await?;

    Ok(rtc)
}

#[embassy_executor::task]
/// Runner for DS3231
///
/// Keeps the time
pub async fn run(rtc_mutex: &'static Mutex<CriticalSectionRawMutex, RtcDS3231>) {
    let sender = TIME_WATCH.sender();

    loop {
        let (datetime, alarm) = {
            let mut rtc = rtc_mutex.lock().await;
            let datetime = rtc.datetime().await.unwrap();
            let alarm = rtc.alarm1().await.unwrap();

            (datetime, alarm)
        };

        sender.send(datetime.into());

        // Listen for alarm requests by web server or GATT
        if IS_ALARM_REQUESTED.load(core::sync::atomic::Ordering::SeqCst) {
            ALARM_SIGNAL.signal(alarm);
        }

        if SET_ALARM.signaled() {
            let config = SET_ALARM.try_take().expect("Already waited signal");
            let mut rtc = rtc_mutex.lock().await;
            rtc.set_alarm1(&config).await.unwrap();
            reset_alarm_flags(&mut rtc).await.unwrap();

            debug!("Set New Alarm: {}", config);

            SET_ALARM.reset();
        }

        #[cfg(debug_assertions)]
        {
            use crate::rtc_ds3231::rtc_time::RtcTime;
            let ts: RtcTime = datetime.into();
            defmt::debug!("{}", ts.to_human());
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
        BUZZER_SIGNAL.signal(BuzzerAction::Toggle);
    }

    BUZZER_SIGNAL.signal(BuzzerAction::Off);

    loop {
        info!("Waiting for alarm...");
        alarm_input.wait_for_falling_edge().await;

        info!("DS3231 Interrupt Received!");
        BUZZER_SIGNAL.signal(BuzzerAction::On);

        #[cfg(debug_assertions)]
        {
            // Stop it from bleeding my ears while devving
            Timer::after_secs(5).await;
            BUZZER_SIGNAL.signal(BuzzerAction::Off);
            info!("Buzzer set low");
        }
    }
}
