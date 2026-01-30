pub mod error;
pub mod rtc_time;
pub use error::*;
pub mod alarm;
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
use esp_hal::i2c::master::I2c;
use static_cell::StaticCell;

use crate::rtc_ds3231::rtc_time::RtcTime;

type I2cAsync = I2c<'static, esp_hal::Async>;

pub static ALARM_CONFIG_RWLOCK: RwLock<CriticalSectionRawMutex, Alarm1Config> =
    RwLock::new(ENV_TIME);

pub static SET_DATETIME_SIGNAL: Signal<CriticalSectionRawMutex, chrono::NaiveDateTime> =
    Signal::new();

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

pub static TIME_WATCH: Watch<CriticalSectionRawMutex, RtcTime, 5> = Watch::new();
pub static SET_ALARM: Signal<CriticalSectionRawMutex, Alarm1Config> = Signal::new();

pub(crate) type RtcDS3231 = DS3231<I2cAsync>;

pub(crate) const RTC_I2C_ADDR: u8 = {
    let addr = option_env!("RTC_I2C_ADDR").unwrap_or(/*0x*/ "68");
    unsafe { u8::from_str_radix(addr, 16).unwrap_unchecked() }
};

// Cannot use RwLock since reading requires &mut self
pub type RtcMutex = Mutex<CriticalSectionRawMutex, RtcDS3231>;
pub static RTC_DS3231: StaticCell<RtcMutex> = StaticCell::new();

/// Initialize the DS3231 Instance and return RTC
pub async fn init_rtc(spawner: Spawner, i2c: I2cAsync) {
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
        .expect("[rtc] Failes to set alarm");
    reset_alarm_flags(&mut rtc)
        .await
        .expect("[rtc] Failed to reset flags");

    *ALARM_CONFIG_RWLOCK.write().await = alarm1_config;

    let rtc_mutex = RTC_DS3231.init(Mutex::new(rtc));

    spawner.must_spawn(run(rtc_mutex));
    spawner.must_spawn(listen_for_clear_flag(rtc_mutex));
    spawner.must_spawn(listen_for_datetime_set(rtc_mutex));
}

pub static CLEAR_FLAGS_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

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

        if SET_ALARM.signaled() {
            let config = SET_ALARM.try_take().unwrap();
            debug!("Set New Alarm: {}", config);

            {
                let mut rtc = rtc_mutex.lock().await;
                rtc.set_alarm1(&config).await.unwrap();
                reset_alarm_flags(&mut rtc).await.unwrap();
            }

            *ALARM_CONFIG_RWLOCK.write().await = config;
            SET_ALARM.reset();
        }

        #[cfg(debug_assertions)]
        {
            if count >= 10 {
                use crate::rtc_ds3231::rtc_time::RtcTime;
                let ts: RtcTime = datetime.into();
                defmt::debug!("{}", ts);
                count = 0;
            }

            count += 1;
        }

        Timer::after_secs(1).await;
    }
}
