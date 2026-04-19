use super::rtc_time::RtcDateTime;
use crate::priority_command::Discriminant;
use chrono::Utc;

#[repr(u8)]
/// RTC Commands.
pub(crate) enum RtcCommand {
    /// Fetches the DS3231 RTC datetime and updates [`TIME_WATCH`](super::TIME_WATCH).
    Tick,
    /// Sets datetime for RTC.
    SetDateTime(RtcDateTime<Utc>),
    /// Sets the RTC module alarm.
    SetAlarm(ds3231::Alarm1Config),
    /// Clears the alarm flags for RTC.
    ClearFlags,
}

// SAFETY: `RtcCommand` is `#[repr(u8)]`.
unsafe impl Discriminant for RtcCommand {}
