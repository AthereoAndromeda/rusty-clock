use super::rtc_time::RtcDateTime;
use chrono::Utc;

#[repr(u8)]
/// RTC Commands with Priority.
///
/// # Priority Levels
/// A lower number indicates higher priority.
///
/// ## Disclaimer
/// The `Ord` and `Eq` implementations only refer to the
/// priority level of each field. It does not check for
/// equality and order for inner data.
pub(crate) enum RtcCommand {
    /// Gets the datetime and updates [`TIME_WATCH`].
    Tick,
    /// Sets datetime for RTC.
    SetDateTime(RtcDateTime<Utc>),
    /// Sets the RTC module alarm.
    SetAlarm(ds3231::Alarm1Config),
    /// Clears the alarm flags for RTC.
    ClearFlags,
}

impl RtcCommand {
    #[inline]
    fn discriminant(&self) -> u8 {
        // SAFETY: Because `Self` is marked `repr(u8)`, its layout is a `repr(C)` `union`
        // between `repr(C)` structs, each of which has the `u8` discriminant as its first
        // field, so we can read the discriminant without offsetting the pointer.
        unsafe { *<*const _>::from(self).cast::<u8>() }
    }
}

impl PartialOrd for RtcCommand {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RtcCommand {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.discriminant().cmp(&other.discriminant())
    }
}

impl PartialEq for RtcCommand {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.discriminant() == other.discriminant()
    }
}

impl Eq for RtcCommand {}
