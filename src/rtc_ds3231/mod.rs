pub mod alarm;
pub mod error;
pub mod rtc_time;
use alarm::*;

mod listener;
use listener::*;

use defmt::debug;
use ds3231::{
    Alarm1Config, Config, DS3231, InterruptControl, Oscillator, SquareWaveFrequency,
    TimeRepresentation,
};
use embassy_executor::Spawner;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex, rwlock::RwLock, signal::Signal,
    watch::Watch,
};
use embassy_time::Timer;

use crate::{i2c::I2cBus, mk_static, rtc_ds3231::rtc_time::RtcTime};

/// The alarm time set through env
const ENV_TIME: Alarm1Config = {
    const HOUR: &str = option_env!("ALARM_HOUR").unwrap_or("0");
    const MIN: &str = option_env!("ALARM_MINUTES").unwrap_or("0");
    const SEC: &str = option_env!("ALARM_SECONDS").unwrap_or("0");

    // SAFETY: Caller is required to guarantee valid number
    const TUP: (u8, u8, u8) = unsafe {
        let h = u8::from_str_radix(HOUR, 10).unwrap_unchecked();
        let m = u8::from_str_radix(MIN, 10).unwrap_unchecked();
        let s = u8::from_str_radix(SEC, 10).unwrap_unchecked();
        (h, m, s)
    };

    // TEST: Valid units
    static_assertions::const_assert!(TUP.0 < 24);
    static_assertions::const_assert!(TUP.1 < 60);
    static_assertions::const_assert!(TUP.2 < 60);

    let (hours, minutes, seconds) = TUP;
    Alarm1Config::AtTime {
        hours,
        minutes,
        seconds,
        is_pm: None,
    }
};

/// Contains the time from RTC module
pub(crate) static TIME_WATCH: Watch<CriticalSectionRawMutex, RtcTime, 3> = Watch::new();
/// Clears the alarm flags for RTC
pub(crate) static CLEAR_FLAGS_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

/// Sets the RTC module alarm
pub(crate) static SET_ALARM: Signal<CriticalSectionRawMutex, Alarm1Config> = Signal::new();

/// Globally accessible Alarm1Config
/// Mostly for Reading the current config. For setting, use [`SET_ALARM`] instead.
pub(crate) static ALARM_CONFIG_RWLOCK: RwLock<CriticalSectionRawMutex, Alarm1Config> =
    RwLock::new(ENV_TIME);

/// Sets datetime for RTC
pub(crate) static SET_DATETIME_SIGNAL: Signal<CriticalSectionRawMutex, chrono::NaiveDateTime> =
    Signal::new();

/// This is the timestamp held in-memory.
/// This is used instead of pinging the RTC module every second
///
/// NOTE: `portable_atomic` crate is used since native does not support 64 bit atomics
pub(crate) static LOCAL_TIMESTAMP: portable_atomic::AtomicU64 = portable_atomic::AtomicU64::new(0);

pub(crate) type RtcDS3231 = DS3231<I2cBus>;
pub(crate) type RtcMutex = Mutex<CriticalSectionRawMutex, RtcDS3231>;

pub(crate) const RTC_I2C_ADDR: u8 = {
    let addr = option_env!("RTC_I2C_ADDR").unwrap_or(/*0x*/ "68");
    unsafe { u8::from_str_radix(addr, 16).unwrap_unchecked() }
};

/// Initialize the DS3231 Instance and spawn tasks
pub(crate) async fn init_rtc(spawner: Spawner, i2c: I2cBus) {
    let config = Config {
        time_representation: TimeRepresentation::TwentyFourHour,
        square_wave_frequency: SquareWaveFrequency::Hz1,
        interrupt_control: InterruptControl::Interrupt,
        battery_backed_square_wave: false,
        oscillator_enable: Oscillator::Enabled,
    };

    let mut rtc = DS3231::new(i2c, RTC_I2C_ADDR);
    rtc.configure(&config)
        .await
        .expect("[rtc] Failed to configure");

    // Hardcoded values for now
    // NOTE: Time stored in RTC is in UTC, adjust to your timezone
    let alarm1_config = if cfg!(debug_assertions) {
        Alarm1Config::AtSeconds { seconds: 30 }
    } else {
        ENV_TIME
    };

    debug!("{:?}", alarm1_config);

    rtc.set_alarm1(&alarm1_config)
        .await
        .expect("[rtc] Failed to set alarm");
    reset_alarm_flags(&mut rtc)
        .await
        .expect("[rtc] Failed to reset flags");

    *ALARM_CONFIG_RWLOCK.write().await = alarm1_config;

    // Cannot use RwLock since reading requires &mut self
    let rtc_mutex = mk_static!(RtcMutex; Mutex::new(rtc));
    spawner.must_spawn(run(rtc_mutex));
    spawner.must_spawn(listen_for_clear_flag(rtc_mutex));
    spawner.must_spawn(listen_for_datetime_set(rtc_mutex));
    spawner.must_spawn(listen_for_alarm_set(rtc_mutex));
}

// TODO: Restructure such that it only gets time at init
// and when requested. saves on cycles
#[embassy_executor::task]
/// Runner for DS3231
///
/// Keeps the time
async fn run(rtc_mutex: &'static RtcMutex) {
    let sender = TIME_WATCH.sender();

    #[cfg(debug_assertions)]
    let mut count = 0;

    loop {
        let datetime = rtc_mutex.lock().await.datetime().await.unwrap();
        sender.send(datetime.into());
        LOCAL_TIMESTAMP.store(
            datetime
                .and_utc()
                .timestamp()
                .try_into()
                .expect("Alarm must never be set before January 1, 1970"),
            core::sync::atomic::Ordering::SeqCst,
        );

        #[cfg(debug_assertions)]
        {
            if count >= 10 {
                use crate::rtc_ds3231::rtc_time::RtcTime;
                let ts: RtcTime = datetime.into();
                defmt::debug!("{}", ts);
                defmt::debug!(
                    "{}",
                    LOCAL_TIMESTAMP.load(core::sync::atomic::Ordering::SeqCst)
                );
                count = 0;
            }

            count += 1;
        }

        Timer::after_secs(1).await;
    }
}
