use super::{CLEAR_FLAGS_SIGNAL, RtcMutex, SET_DATETIME_SIGNAL, reset_alarm_flags_mutex};

#[embassy_executor::task]
pub(super) async fn listen_for_datetime_set(rtc: &'static RtcMutex) {
    loop {
        let datetime = SET_DATETIME_SIGNAL.wait().await;

        match rtc.lock().await.set_datetime(&datetime).await {
            Ok(_) => {}
            Err(e) => defmt::error!(
                "[rtc] Failed to set new datetime: {}",
                defmt::Debug2Format(&e)
            ),
        };
    }
}

#[embassy_executor::task]
pub(super) async fn listen_for_clear_flag(rtc: &'static RtcMutex) {
    loop {
        CLEAR_FLAGS_SIGNAL.wait().await;

        match reset_alarm_flags_mutex(rtc).await {
            Ok(_) => {}
            Err(e) => defmt::error!("[rtc] Failed to reset flags: {}", defmt::Debug2Format(&e)),
        };
    }
}
