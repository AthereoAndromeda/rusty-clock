//! # DS3231 RTC Listeners
//! This module provides tasks related to our RTC module.
//!

use crate::rtc_ds3231::{ALARM_CONFIG_RWLOCK, SET_ALARM, reset_alarm_flags};

use super::{CLEAR_FLAGS_SIGNAL, RtcMutex, SET_DATETIME_SIGNAL};

#[embassy_executor::task]
pub(super) async fn listen_for_datetime_set(rtc: &'static RtcMutex) -> ! {
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
pub(super) async fn listen_for_clear_flag(rtc: &'static RtcMutex) -> ! {
    loop {
        CLEAR_FLAGS_SIGNAL.wait().await;

        let mut rtc = rtc.lock().await;
        if let Err(err) = reset_alarm_flags(&mut rtc).await {
            defmt::error!("[rtc] Failed to reset flags: {}", defmt::Debug2Format(&err));
        }
    }
}

#[embassy_executor::task]
pub(super) async fn listen_for_alarm_set(rtc: &'static RtcMutex) -> ! {
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
