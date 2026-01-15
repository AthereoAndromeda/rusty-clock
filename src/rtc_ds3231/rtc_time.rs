use core::ops::Deref;

use chrono::{Datelike, NaiveDateTime, Timelike};

#[derive(Debug, Copy, Clone)]
/// A wrapper around `chrono::NaiveDateTime` and also implements `Deref`
pub struct RtcTime(pub NaiveDateTime);

impl Deref for RtcTime {
    type Target = NaiveDateTime;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<chrono::NaiveDateTime> for RtcTime {
    fn from(value: chrono::NaiveDateTime) -> Self {
        Self(value)
    }
}

impl From<RtcTime> for chrono::NaiveDateTime {
    fn from(value: RtcTime) -> Self {
        value.0
    }
}

impl defmt::Format for RtcTime {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(
            fmt,
            "{}-{}-{}T{}:{}:{}",
            self.0.year(),
            self.0.month(),
            self.0.day(),
            self.0.hour(),
            self.0.minute(),
            self.0.second()
        )
    }
}
