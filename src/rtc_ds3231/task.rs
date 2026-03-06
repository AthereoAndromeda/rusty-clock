//! # DS3231 RTC Tasks
//! This module provides tasks related to our RTC module.

use chrono::Utc;
use embassy_time::Timer;

use super::{
    ALARM_CONFIG_RWLOCK, CLEAR_FLAGS_SIGNAL, LOCAL_TIMESTAMP, RtcMutex, SET_ALARM,
    SET_DATETIME_SIGNAL, TIME_WATCH, reset_alarm_flags, rtc_time::RtcDateTime,
};

// TODO: Restructure such that it only gets time at init
// and when requested. saves on cycles
#[embassy_executor::task]
/// Runner for DS3231.
///
/// Keeps the time.
pub(super) async fn runner_task(rtc_mutex: &'static RtcMutex) -> ! {
    let sender = TIME_WATCH.sender();

    #[cfg(debug_assertions)]
    let mut count = 0;

    loop {
        let datetime: RtcDateTime<Utc> = rtc_mutex
            .lock()
            .await
            .datetime()
            .await
            .expect("Failed to retrieve RTC datetime data")
            .and_utc()
            .into();

        let ts = datetime.timestamp();
        sender.send(datetime);

        assert!(
            ts.is_positive(),
            "The timestamp should never be negative, i.e. never set before January 1 1970"
        );
        let ts = ts.cast_unsigned();

        LOCAL_TIMESTAMP.store(ts, core::sync::atomic::Ordering::Release);

        #[cfg(debug_assertions)]
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "Not expected to overflow in debug builds"
        )]
        {
            // Only print time every 10 seconds instead of every second
            if count >= 10 {
                defmt::debug!("Local: {=str}", datetime.local().to_iso8601());
                defmt::debug!("Local: {=str}", datetime.local().to_human());
                defmt::debug!("UTC  : {=str}", datetime.to_iso8601());
                defmt::debug!("UTC  : {=str}", datetime.to_human());
                defmt::debug!("TS   : {=u64}", ts);
                count = 0;
            }

            count += 1;
        }

        Timer::after_secs(1).await;
    }
}

#[embassy_executor::task]
/// Sets the RTC module to the received datetime.
pub(super) async fn datetime_task(rtc: &'static RtcMutex) -> ! {
    loop {
        let datetime = SET_DATETIME_SIGNAL.wait().await;

        if let Err(err) = rtc.lock().await.set_datetime(&datetime.naive_utc()).await {
            defmt::error!(
                "[rtc] Failed to set new datetime: {}",
                defmt::Debug2Format(&err)
            );
        }
    }
}

#[embassy_executor::task]
/// Clears RTC alarm flags.
pub(super) async fn flag_task(rtc: &'static RtcMutex) -> ! {
    loop {
        CLEAR_FLAGS_SIGNAL.wait().await;

        let mut rtc = rtc.lock().await;
        if let Err(err) = reset_alarm_flags(&mut rtc).await {
            defmt::error!("[rtc] Failed to reset flags: {}", defmt::Debug2Format(&err));
        }
    }
}

#[embassy_executor::task]
/// Sets the RTC to the received alarm.
pub(super) async fn alarm_task(rtc: &'static RtcMutex) -> ! {
    loop {
        let config = SET_ALARM.wait().await;
        defmt::info!("New Alarm Set: {}", config);

        {
            let mut rtc = rtc.lock().await;
            rtc.set_alarm1(&config).await.unwrap();
            if let Err(err) = reset_alarm_flags(&mut rtc).await {
                defmt::error!("[rtc] Failed to reset flags: {}", defmt::Debug2Format(&err));
                continue;
            }
        }

        *ALARM_CONFIG_RWLOCK.write().await = config;
    }
}
