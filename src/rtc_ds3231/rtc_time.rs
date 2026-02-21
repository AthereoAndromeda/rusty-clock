//! # `RtcDateTime`
//! This module provides all functionalities regarding [`RtcDateTime`].

use chrono::{Datelike, FixedOffset, NaiveDateTime, TimeZone as _, Timelike};
use core::{fmt::Debug, hint::assert_unchecked, ops::Deref};

use crate::TZ_OFFSET;

#[derive(Debug, Copy, Clone)]
/// A wrapper around [`chrono::NaiveDateTime`].
///
/// This wrapper implements `Deref`. This wrapper also provides
/// convenience methods, impls, and interfaces wih our web server.
pub(crate) struct RtcDateTime(pub NaiveDateTime);

impl RtcDateTime {
    /// Converts [`RtcDateTime`] to a human-readable format.
    pub fn to_human(self) -> HumanDateTime {
        HumanDateTime(self.0)
    }

    /// Converts [`RtcDateTime`] to conform to ISO8601.
    pub fn to_iso8601(self) -> Iso8601DateTime {
        Iso8601DateTime(self.0)
    }

    /// Returns seconds since Unix Epoch.
    pub fn to_timestamp(self) -> u64 {
        self.0.and_utc().timestamp().cast_unsigned()
    }

    /// Generate a [`RtcDateTime`] from seconds since Unix Epoch.
    pub fn from_timestamp(ts: i64) -> Self {
        chrono::Utc.timestamp_opt(ts, 0).unwrap().naive_utc().into()
    }
}

impl Deref for RtcDateTime {
    type Target = NaiveDateTime;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<chrono::NaiveDateTime> for RtcDateTime {
    fn from(value: chrono::NaiveDateTime) -> Self {
        Self(value)
    }
}

impl From<RtcDateTime> for chrono::NaiveDateTime {
    fn from(value: RtcDateTime) -> Self {
        value.0
    }
}

impl From<i64> for RtcDateTime {
    fn from(value: i64) -> Self {
        RtcDateTime::from_timestamp(value)
    }
}

// impl defmt::Format for RtcDateTime {
//     fn format(&self, fmt: defmt::Formatter) {
//         defmt::write!(fmt, "{=str}", self.to_iso8601().local());
//     }
// }

impl picoserve::response::sse::EventData for RtcDateTime {
    async fn write_to<W: picoserve::io::Write>(self, writer: &mut W) -> Result<(), W::Error> {
        writer
            .write_all(self.to_human().local_short().as_bytes())
            .await?;
        Ok(())
    }
}

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

#[inline]
const fn get_shorthand(month: &str) -> &str {
    &month[..3]
}

/// Provides method to format date to a human readable form.
pub(crate) struct HumanDateTime(NaiveDateTime);

impl HumanDateTime {
    #[expect(clippy::indexing_slicing, reason = "Guaranteed to be < 12")]
    fn to_human_inner<const N: usize>(
        dt: &(impl Datelike + Timelike),
        use_full_month: bool,
    ) -> heapless::String<N> {
        let month_idx = dt.month0() as usize;

        // SAFETY: There is always 12 months. If not, something has gone
        // terribly wrong or you have transported to a parallel dimension
        unsafe {
            assert_unchecked(month_idx < 12);
        }

        let month = if use_full_month {
            MONTH_BY_INDEX[month_idx]
        } else {
            get_shorthand(MONTH_BY_INDEX[month_idx])
        };

        // TODO: Shorten Months and add Day of the week
        heapless::format!(
            "{:02}:{:02}:{:02} | {:02} {} {}",
            dt.hour(),
            dt.minute(),
            dt.second(),
            dt.day(),
            month,
            dt.year(),
        )
        .unwrap()
    }

    // Longest possible string is 28
    #[expect(unused, reason = "Use later")]
    pub fn local(&self) -> heapless::String<28> {
        let time = self
            .0
            .and_local_timezone(FixedOffset::east_opt(i32::from(TZ_OFFSET) * 3600).unwrap())
            .unwrap();

        Self::to_human_inner(&time, true)
    }

    // String is always 22 characters long
    pub fn local_short(&self) -> heapless::String<22> {
        let time = self
            .0
            .and_local_timezone(FixedOffset::east_opt(i32::from(TZ_OFFSET) * 3600).unwrap())
            .unwrap();

        Self::to_human_inner(&time, false)
    }

    #[expect(unused, reason = "Use later")]
    pub fn utc(&self) -> heapless::String<50> {
        Self::to_human_inner(&self.0, true)
    }

    #[expect(unused, reason = "Use later")]
    pub fn utc_short(&self) -> heapless::String<50> {
        Self::to_human_inner(&self.0, false)
    }
}

/// Provides methods to format to `ISO 8601` format.
pub(crate) struct Iso8601DateTime(NaiveDateTime);

impl Iso8601DateTime {
    #[expect(unused, reason = "Use later")]
    pub fn utc(&self) -> heapless::String<20> {
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

    pub fn local(&self) -> heapless::String<25> {
        let time = self
            .0
            .and_local_timezone(FixedOffset::east_opt(i32::from(TZ_OFFSET) * 3600).unwrap())
            .unwrap();

        let sign = if TZ_OFFSET.is_positive() { "+" } else { "-" };

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
}
