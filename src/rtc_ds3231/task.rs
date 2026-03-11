//! # DS3231 RTC Tasks
//! This module provides tasks related to our RTC module.

use chrono::Utc;
use ds3231::Alarm1Config;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Sender};
use embassy_time::Timer;

use super::{
    ALARM_CONFIG_RWLOCK, CLEAR_FLAGS_SIGNAL, LOCAL_TIMESTAMP, RtcDS3231, SET_ALARM,
    SET_DATETIME_SIGNAL, TIME_WATCH, reset_alarm_flags, rtc_time::RtcDateTime,
};

#[embassy_executor::task]
pub(super) async fn runner(mut rtc: RtcDS3231) -> ! {
    use embassy_futures::select::{Either4, select4};
    let sender = TIME_WATCH.sender();
    #[cfg(debug_assertions)]
    let mut count = 0;

    loop {
        let task = select4(
            Timer::after_secs(1),
            SET_DATETIME_SIGNAL.wait(),
            SET_ALARM.wait(),
            CLEAR_FLAGS_SIGNAL.wait(),
        )
        .await;

        match task {
            Either4::First(()) => {
                time_handle(
                    &sender,
                    &mut rtc,
                    #[cfg(debug_assertions)]
                    &mut count,
                )
                .await;
            }
            Either4::Second(datetime) => set_datetime_handle(&mut rtc, datetime).await,
            Either4::Third(alarm) => alarm_handle(&mut rtc, alarm).await,
            Either4::Fourth(()) => clear_flags_handle(&mut rtc).await,
        }
    }
}

async fn clear_flags_handle(rtc: &mut RtcDS3231) {
    if let Err(err) = reset_alarm_flags(rtc).await {
        defmt::error!("[rtc] Failed to reset flags: {}", defmt::Debug2Format(&err));
    }
}

async fn alarm_handle(rtc: &mut RtcDS3231, config: Alarm1Config) {
    defmt::info!("New Alarm Set: {}", config);

    if let Err(err) = rtc.set_alarm1(&config).await {
        defmt::error!("[rtc] Failed to set Alarm1: {}", defmt::Debug2Format(&err));
        return;
    }

    if let Err(err) = reset_alarm_flags(rtc).await {
        defmt::error!("[rtc] Failed to reset flags: {}", defmt::Debug2Format(&err));
        return;
    }

    *ALARM_CONFIG_RWLOCK.write().await = config;
}

async fn set_datetime_handle(rtc: &mut RtcDS3231, datetime: RtcDateTime<Utc>) {
    if let Err(err) = rtc.set_datetime(&datetime.naive_utc()).await {
        defmt::error!(
            "[rtc] Failed to set new datetime: {}",
            defmt::Debug2Format(&err)
        );
    }
}

// TODO: Restructure such that it only gets time at init
// and when requested. saves on cycles
async fn time_handle(
    sender: &Sender<'_, CriticalSectionRawMutex, RtcDateTime<Utc>, 3>,
    rtc: &mut RtcDS3231,
    #[cfg(debug_assertions)] count: &mut usize,
) {
    let datetime: RtcDateTime<Utc> = rtc.datetime().await.unwrap().and_utc().into();
    sender.send(datetime);

    let ts = datetime.timestamp();
    defmt::assert!(
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
        if *count >= 10 {
            defmt::debug!("Local: {=str}", datetime.local().to_iso8601());
            defmt::debug!("Local: {=str}", datetime.local().to_human());
            defmt::debug!("UTC  : {=str}", datetime.to_iso8601());
            defmt::debug!("UTC  : {=str}", datetime.to_human());
            defmt::debug!("TS   : {=u64}", ts);
            *count = 0;
        }

        *count += 1;
    }
}
