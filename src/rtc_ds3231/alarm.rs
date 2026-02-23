use super::{RtcDS3231, error::RtcError};
use defmt::debug;

/// Clears and Sets Alarm1 Flag.
pub(super) async fn reset_alarm_flags(rtc: &mut RtcDS3231) -> Result<(), RtcError> {
    let mut status = rtc.status().await?;
    status.set_alarm1_flag(false);
    status.set_alarm2_flag(false);
    rtc.set_status(status).await?;

    #[cfg(debug_assertions)]
    debug!("[rtc:init] Alarm flags cleared");

    // Enable Alarm 1 interrupt
    let mut control = rtc.control().await?;
    control.set_alarm1_interrupt_enable(true);
    control.set_alarm2_interrupt_enable(false);
    rtc.set_control(control).await?;

    #[cfg(debug_assertions)]
    debug!("[rtc:init] Alarm 1 interrupt enabled");
    Ok(())
}
