use crate::rtc_ds3231::{RtcDS3231, RtcError, RtcMutex};
use defmt::info;

/// Clears and Sets Alarm1 Flag
pub(super) async fn reset_alarm_flags(rtc: &mut RtcDS3231) -> Result<(), RtcError> {
    let mut status = rtc.status().await?;
    status.set_alarm1_flag(false);
    status.set_alarm2_flag(false);
    rtc.set_status(status).await?;

    info!("[rtc:init] Alarm flags cleared");

    // Enable Alarm 1 interrupt
    let mut control = rtc.control().await?;
    control.set_alarm1_interrupt_enable(true);
    control.set_alarm2_interrupt_enable(false);
    rtc.set_control(control).await?;

    info!("[rtc:init] Alarm 1 interrupt enabled");
    Ok(())
}

pub(super) async fn reset_alarm_flags_mutex(rtc: &RtcMutex) -> Result<(), RtcError> {
    let mut rtc = rtc.lock().await;
    let mut status = rtc.status().await?;
    status.set_alarm1_flag(false);
    status.set_alarm2_flag(false);
    rtc.set_status(status).await?;

    info!("[rtc:init] Alarm flags cleared");

    // Enable Alarm 1 interrupt
    let mut control = rtc.control().await?;
    control.set_alarm1_interrupt_enable(true);
    control.set_alarm2_interrupt_enable(false);
    rtc.set_control(control).await?;

    info!("[rtc:init] Alarm 1 interrupt enabled");
    Ok(())
}
