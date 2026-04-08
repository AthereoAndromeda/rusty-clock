use super::{RtcDS3231, error::RtcError};

/// Clears and Sets Alarm1 Flag.
pub(super) async fn reset_alarm1_flags(rtc: &mut RtcDS3231) -> Result<(), RtcError> {
    let mut status = rtc.status().await?;
    status.set_alarm1_flag(false);
    rtc.set_status(status).await?;

    #[cfg(debug_assertions)]
    defmt::debug!("[rtc:init] Alarm 1 flag cleared");

    // Enable Alarm 1 interrupt
    let mut control = rtc.control().await?;
    control.set_alarm1_interrupt_enable(true);
    rtc.set_control(control).await?;

    #[cfg(debug_assertions)]
    defmt::debug!("[rtc:init] Alarm 1 interrupt enabled");
    Ok(())
}
