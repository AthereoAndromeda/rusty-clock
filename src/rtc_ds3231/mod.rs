//! # DS3231 RTC
//! This module provides implementations for interfacing with our
//! DS3231 Real Time Clock module.
//!
//! The module also provides access to [`embassy_sync`] primitives
//! that interface with the RTC module.

pub mod alarm;
pub(crate) mod command;
pub mod error;
pub mod rtc_time;
mod task;
use alarm::reset_alarm1_flags;
pub(crate) use command::RtcCommand;
use rtc_time::RtcDateTime;

use chrono::Utc;
use ds3231::{
    Alarm1Config, Config, DS3231, InterruptControl, Oscillator, SquareWaveFrequency,
    TimeRepresentation,
};
use embassy_executor::Spawner;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    priority_channel::{Min, PriorityChannel},
    rwlock::RwLock,
    watch::Watch,
};

use crate::i2c::I2cBus;

/// The alarm time set through env.
/// NOTE: Time stored in RTC is in UTC, adjust to your timezone
const ENV_TIME: Alarm1Config = {
    const HOUR: &str = option_env!("ALARM_HOUR").unwrap_or("0");
    const MIN: &str = option_env!("ALARM_MINUTES").unwrap_or("0");
    const SEC: &str = option_env!("ALARM_SECONDS").unwrap_or("0");

    const TUP: (u8, u8, u8) = {
        let h = u8::from_str_radix(HOUR, 10)
            .ok()
            .expect("Failed to parse .env: ALARM_HOUR");
        let m = u8::from_str_radix(MIN, 10)
            .ok()
            .expect("Failed to parse .env: ALARM_MINUTES");
        let s = u8::from_str_radix(SEC, 10)
            .ok()
            .expect("Failed to parse .env: ALARM_SECONDS");
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

/// Contains the time from RTC module.
pub(crate) static TIME_WATCH: Watch<CriticalSectionRawMutex, RtcDateTime<Utc>, 3> = Watch::new();

/// Globally accessible [`Alarm1Config`].
pub(crate) static ALARM_CONFIG_RWLOCK: RwLock<CriticalSectionRawMutex, Alarm1Config> =
    RwLock::new(ENV_TIME);

/// The inbox for all RTC Commands.
pub(crate) static RTC_COMMANDS: PriorityChannel<CriticalSectionRawMutex, RtcCommand, Min, 4> =
    PriorityChannel::new();

type RtcDS3231 = DS3231<I2cBus>;

pub(crate) const RTC_I2C_ADDR: u8 = {
    let addr = option_env!("RTC_I2C_ADDR").unwrap_or(/*0x*/ "68");
    u8::from_str_radix(addr, 16)
        .ok()
        .expect("Failed to parse .env: RTC_I2C_ADDR")
};

/// Initialize the DS3231 Instance and spawn tasks.
pub(crate) async fn init(spawner: Spawner, i2c: I2cBus) {
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

    #[cfg(debug_assertions)]
    {
        // Only set alarm in debug builds. Uses previously set alarm in production.
        defmt::debug!("Alarm1 Config: {:?}", ENV_TIME);
        *ALARM_CONFIG_RWLOCK.write().await = ENV_TIME;
        rtc.set_alarm1(&ENV_TIME)
            .await
            .expect("[rtc] Failed to set alarm");
    }

    reset_alarm1_flags(&mut rtc)
        .await
        .expect("[rtc] Failed to reset flags");

    spawner.must_spawn(task::runner(rtc));
    spawner.must_spawn(task::heartbeat_task());
}
