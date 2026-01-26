use core::{fmt::Debug, ops::Deref};

use chrono::{Datelike, FixedOffset, NaiveDateTime, TimeZone, Timelike};

use crate::TZ_OFFSET;

#[derive(Debug, Copy, Clone)]
/// A wrapper around `chrono::NaiveDateTime` and also implements `Deref`
pub struct RtcTime(pub NaiveDateTime);

const MONTH_BY_INDEX: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

impl RtcTime {
    fn human_inner(dt: impl Datelike + Timelike) -> heapless::String<50> {
        heapless::format!(
            "{} {} {:02} | {:02}:{:02}:{:02}",
            dt.year(),
            MONTH_BY_INDEX[dt.month() as usize],
            dt.day(),
            dt.hour(),
            dt.minute(),
            dt.second(),
        )
        .unwrap()
    }

    pub fn to_human_utc(self) -> heapless::String<50> {
        Self::human_inner(self.0)
    }

    pub fn to_human_local(self) -> heapless::String<50> {
        let time = self
            .0
            .and_local_timezone(FixedOffset::east_opt(TZ_OFFSET as i32 * 3600).unwrap())
            .unwrap();

        Self::human_inner(time)
    }

    pub fn to_iso8601_utc(self) -> heapless::String<20> {
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

    pub fn to_iso8601_local(self) -> heapless::String<25> {
        let time = self
            .0
            .and_local_timezone(FixedOffset::east_opt(TZ_OFFSET as i32 * 3600).unwrap())
            .unwrap();

        let sign = if TZ_OFFSET >= 0 { "+" } else { "-" };

        heapless::format!(
            "{}-{:02}-{:02}T{:02}:{:02}:{:02}{}{:02}:00",
            time.year(),
            time.month(),
            time.day(),
            time.hour(),
            time.minute(),
            time.second(),
            sign,
            TZ_OFFSET
        )
        .unwrap()
    }

    pub fn from_timestamp(ts: i64) -> Self {
        chrono::Utc.timestamp_opt(ts, 0).unwrap().naive_utc().into()
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

impl From<i64> for RtcTime {
    fn from(value: i64) -> Self {
        RtcTime::from_timestamp(value)
    }
}

impl defmt::Format for RtcTime {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "{}", self.to_iso8601_local());
    }
}
