use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};

#[derive(Debug, Copy, Clone)]
/// Bluetooth-compatible time struct
///
/// # WARNING
/// Assumes year won't be larger than u16,
/// and all other fields wont be bigger than 255
pub struct RtcTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

// I'll keep this here in case i need it
// #[derive(Debug, Clone, Copy, thiserror::Error)]
// pub enum RtcTimeError {
//     #[error("Issue coercing NaiveDateTime to RtcTime")]
//     CoercionError(#[from] TryFromIntError),
// }

// impl TryFrom<chrono::NaiveDateTime> for RtcTime {
//     type Error = RtcTimeError;
//     fn try_from(value: chrono::NaiveDateTime) -> Result<Self, Self::Error> {
//         Ok(Self {
//             year: value.year().try_into()?,
//             month: value.month().try_into()?,
//             day: value.day().try_into()?,
//             hour: value.hour().try_into()?,
//             minute: value.minute().try_into()?,
//             second: value.second().try_into()?,
//         })
//     }
// }

impl From<chrono::NaiveDateTime> for RtcTime {
    fn from(value: chrono::NaiveDateTime) -> Self {
        Self {
            year: value.year().try_into().unwrap(),
            month: value.month().try_into().unwrap(),
            day: value.day().try_into().unwrap(),
            hour: value.hour().try_into().unwrap(),
            minute: value.minute().try_into().unwrap(),
            second: value.second().try_into().unwrap(),
        }
    }
}
impl From<RtcTime> for chrono::NaiveDateTime {
    fn from(value: RtcTime) -> Self {
        let date = NaiveDate::from_ymd_opt(value.year.into(), value.month.into(), value.day.into())
            .unwrap();
        let time =
            NaiveTime::from_hms_opt(value.hour.into(), value.day.into(), value.second.into())
                .unwrap();

        NaiveDateTime::new(date, time)
    }
}

impl defmt::Format for RtcTime {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(
            fmt,
            "{}-{}-{}T{}:{}:{}",
            self.year,
            self.month,
            self.day,
            self.hour,
            self.minute,
            self.second
        )
    }
}
