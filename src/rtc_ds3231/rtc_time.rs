//! # `RtcDateTime`
//! This module provides all functionalities regarding [`RtcDateTime`].

use chrono::{DateTime, Datelike, FixedOffset, TimeZone, Timelike, Utc};
use core::{fmt::Debug, hint::assert_unchecked, ops::Deref};

use crate::TZ_OFFSET;

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

#[derive(Debug, Clone)]
/// A wrapper around [`chrono::NaiveDateTime`].
///
/// This wrapper implements `Deref`. This wrapper also provides
/// convenience methods, impls, and interfaces wih our web server.
pub(crate) struct RtcDateTime<TZ: TimeZone + Copy>(pub DateTime<TZ>);

impl<TZ: TimeZone + Copy> RtcDateTime<TZ> {
    #[expect(clippy::indexing_slicing, reason = "Guaranteed to be < 12")]
    fn to_human_inner<const N: usize>(
        dt: &(impl Datelike + Timelike),
        use_full_month: bool,
    ) -> heapless::String<N> {
        #[expect(
            clippy::as_conversions,
            reason = "Month is guaranteed to fit inside of `usize`"
        )]
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

    #[inline]
    /// Converts [`RtcDateTime`] to a human-readable format.
    pub fn to_human(&self) -> heapless::String<28> {
        Self::to_human_inner(&self.0, true)
    }

    #[inline]
    /// Converts [`RtcDateTime`] to a human-readable format with 3-letter month.
    pub fn to_human_short(&self) -> heapless::String<22> {
        Self::to_human_inner(&self.0, false)
    }

    #[inline]
    /// Returns seconds since Unix Epoch.
    pub fn to_timestamp(&self) -> u64 {
        self.0.timestamp().cast_unsigned()
    }

    #[inline]
    /// Generate a [`RtcDateTime`] from seconds since Unix Epoch.
    pub fn from_timestamp(ts: i64) -> RtcDateTime<Utc> {
        chrono::Utc.timestamp_opt(ts, 0).unwrap().into()
    }
}

impl RtcDateTime<Utc> {
    #[inline]
    /// Converts itself to `Local` variant.
    pub fn local(self) -> RtcDateTime<FixedOffset> {
        let offset = FixedOffset::east_opt(i32::from(TZ_OFFSET) * 3600).unwrap();
        let time = self.0.with_timezone(&offset);
        RtcDateTime(time)
    }

    #[inline]
    /// Converts [`RtcDateTime`] to ISO8601-conformant string.
    pub fn to_iso8601(self) -> heapless::String<20> {
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

impl RtcDateTime<FixedOffset> {
    #[inline]
    #[expect(unused, reason = "Will use later")]
    /// Converts itself to `Utc` variant.
    pub fn utc(self) -> RtcDateTime<Utc> {
        RtcDateTime(self.0.to_utc())
    }

    #[inline]
    /// Converts [`RtcDateTime`] to ISO8601-conformant string.
    pub fn to_iso8601(self) -> heapless::String<25> {
        let sign = if TZ_OFFSET.is_positive() { "+" } else { "-" };

        heapless::format!(
            "{}-{:02}-{:02}T{:02}:{:02}:{:02}{}{:02}:00",
            self.0.year(),
            self.0.month(),
            self.0.day(),
            self.0.hour(),
            self.0.minute(),
            self.0.second(),
            sign,
            TZ_OFFSET
        )
        .unwrap()
    }
}

impl<TZ: TimeZone + Copy> Deref for RtcDateTime<TZ> {
    type Target = DateTime<TZ>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<TZ: TimeZone + Copy> From<DateTime<TZ>> for RtcDateTime<TZ> {
    #[inline]
    fn from(value: DateTime<TZ>) -> Self {
        Self(value)
    }
}

impl<TZ: TimeZone + Copy> From<RtcDateTime<TZ>> for DateTime<TZ> {
    #[inline]
    fn from(value: RtcDateTime<TZ>) -> Self {
        value.0
    }
}

impl<TZ: TimeZone + Copy> picoserve::response::sse::EventData for RtcDateTime<TZ> {
    async fn write_to<W: picoserve::io::Write>(self, writer: &mut W) -> Result<(), W::Error> {
        writer.write_all(self.to_human_short().as_bytes()).await?;
        Ok(())
    }
}

impl<TZ: TimeZone + Copy> Copy for RtcDateTime<TZ> where
    <TZ as TimeZone>::Offset: Copy // NaiveDateTime: Copy,
{
}
