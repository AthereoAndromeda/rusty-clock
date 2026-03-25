//! # DS3231 RTC Tasks
//! This module provides tasks related to our RTC module.

use chrono::Utc;
use ds3231::Alarm1Config;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Sender};
use embassy_time::Timer;

use super::{
    ALARM_CONFIG_RWLOCK, LOCAL_TIMESTAMP, RTC_COMMANDS, RtcCommand, RtcDS3231, TIME_WATCH,
    reset_alarm_flags, rtc_time::RtcDateTime,
};

#[embassy_executor::task]
pub(super) async fn runner(mut rtc: RtcDS3231) -> ! {
    let time_sender = TIME_WATCH.sender();
    let cmd_rx = RTC_COMMANDS.receiver();
    #[cfg(debug_assertions)]
    let mut count = 0;

    loop {
        let rtc = &mut rtc;
        match cmd_rx.receive().await {
            RtcCommand::Tick => {
                time_handle(
                    &time_sender,
                    rtc,
                    #[cfg(debug_assertions)]
                    &mut count,
                )
                .await;
            }
            RtcCommand::SetDateTime(datetime) => set_datetime_handle(rtc, datetime).await,
            RtcCommand::SetAlarm(config) => alarm_handle(rtc, config).await,
            RtcCommand::ClearFlags => clear_flags_handle(rtc).await,
        }
    }
}

#[embassy_executor::task]
pub(super) async fn heartbeat_task() -> ! {
    let sender = RTC_COMMANDS.sender();
    loop {
        sender.send(RtcCommand::Tick).await;
        Timer::after_secs(1).await;
    }
}

#[inline]
async fn clear_flags_handle(rtc: &mut RtcDS3231) {
    if let Err(err) = reset_alarm_flags(rtc).await {
        defmt::error!("[rtc] Failed to reset flags: {}", defmt::Debug2Format(&err));
    }
}

#[inline]
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

#[inline]
async fn set_datetime_handle(rtc: &mut RtcDS3231, datetime: RtcDateTime<Utc>) {
    if let Err(err) = rtc.set_datetime(&datetime.naive_utc()).await {
        defmt::error!(
            "[rtc] Failed to set new datetime: {}",
            defmt::Debug2Format(&err)
        );
    }
}

#[inline]
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
