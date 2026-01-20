use core::{fmt::Debug, ops::Deref};

use chrono::{Datelike, FixedOffset, NaiveDateTime, Timelike};

use crate::TZ_OFFSET;

#[derive(Debug, Copy, Clone)]
/// A wrapper around `chrono::NaiveDateTime` and also implements `Deref`
pub struct RtcTime(pub NaiveDateTime);

impl RtcTime {
    pub fn to_human_utc(&self) -> heapless::String<30> {
        heapless::format!(
            30;
            "{}-{:02}-{:02} | {:02}:{:02}:{:02} (00:00)",
            self.0.year(),
            self.0.month(),
            self.0.day(),
            self.0.hour(),
            self.0.minute(),
            self.0.second(),
        )
        .unwrap()
    }

    pub fn to_human_local(&self) -> heapless::String<30> {
        let time = self
            .0
            .and_local_timezone(FixedOffset::east_opt(TZ_OFFSET as i32 * 3600).unwrap())
            .unwrap();

        heapless::format!(
            30;
            "{}-{:02}-{:02} | {:02}:{:02}:{:02} ({:02}:00)",
            time.year(),
            time.month(),
            time.day(),
            time.hour(),
            time.minute(),
            time.second(),
            TZ_OFFSET
        )
        .unwrap()
    }

    pub fn to_iso8601_utc(&self) -> heapless::String<20> {
        heapless::format!(
            "{}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            self.0.year(),
            self.0.month(),
            self.0.day(),
            self.0.hour(),
            self.0.minute(),
            self.0.second(),
        )
        .unwrap()
    }
}

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
