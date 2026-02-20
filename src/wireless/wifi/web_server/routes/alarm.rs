use chrono::Timelike as _;
use ds3231::Alarm1Config;
use explicit_cast::prelude::*;
use picoserve::{
    Router,
    extract::{Form, Json, Query},
    response::{DebugValue, IntoResponse},
    routing::{PathRouter, get, parse_path_segment, post},
};
use serde::Deserialize;

use crate::{
    TZ_OFFSET,
    rtc_ds3231::{ALARM_CONFIG_RWLOCK, CLEAR_FLAGS_SIGNAL, SET_ALARM},
};

pub(super) fn add_routes(router: Router<impl PathRouter>) -> Router<impl PathRouter> {
    router
        .route("/alarm", get(get_alarm))
        .route("/alarm/clear", get(get_clear_flags))
        .route("/alarm/submit", post(set_alarm_form))
        .route("/alarm/json", post(set_alarm_json))
        .route(
            (
                "/alarm",
                parse_path_segment::<u8>(),
                parse_path_segment::<u8>(),
                parse_path_segment::<u8>(),
            ),
            get(set_alarm),
        )
}

#[derive(Debug, Deserialize)]
struct AlarmQueryParams {
    pub utc: Option<bool>,
}

async fn get_alarm() -> impl IntoResponse {
    let response = ALARM_CONFIG_RWLOCK.read().await;
    DebugValue(response)
}

fn set_alarm_inner(hour: u8, min: u8, sec: u8, is_utc: bool) {
    let base_time =
        chrono::NaiveTime::from_hms_opt(u32::from(hour), u32::from(min), u32::from(sec)).unwrap();

    let time = if is_utc {
        base_time
    } else {
        base_time
            .overflowing_sub_signed(chrono::TimeDelta::hours(TZ_OFFSET.into()))
            .0
    };

    #[cfg(debug_assertions)]
    defmt::debug!(
        "{=u32}:{=u32}:{=u32}",
        time.hour(),
        time.minute(),
        time.second()
    );

    let conf = Alarm1Config::AtTime {
        hours: time.hour().truncate(),
        minutes: time.minute().truncate(),
        seconds: time.second().truncate(),
        is_pm: None,
    };

    SET_ALARM.signal(conf);
}

async fn set_alarm(
    (hour, min, sec): (u8, u8, u8),
    Query(query): Query<AlarmQueryParams>,
) -> impl IntoResponse {
    set_alarm_inner(hour, min, sec, query.utc.is_some_and(|x| x));
    "Alarm Set!"
}

#[derive(Debug, Deserialize, defmt::Format)]
struct AlarmForm {
    pub hour: u8,
    pub min: u8,
    pub sec: u8,
    pub is_utc: heapless::String<3>,
}

async fn set_alarm_form(Form(form): Form<AlarmForm>) -> impl IntoResponse {
    defmt::debug!("{}", &form);
    let AlarmForm {
        hour,
        min,
        sec,
        is_utc,
    } = form;

    set_alarm_inner(hour, min, sec, is_utc == "on");
}

/// Alarm 1 specific configurations.
/// 1-to-1 mapping to [`Alarm1Config`], but with serde.
#[derive(Debug, Clone, PartialEq, Deserialize, defmt::Format)]
enum MyAlarm1Config {
    /// Trigger every second (all mask bits set).
    EverySecond,

    /// Trigger when seconds match (A1M1=0, others=1).
    AtSeconds {
        /// Seconds value (0-59).
        seconds: u8,
    },

    /// Trigger when minutes and seconds match (A1M1=0, A1M2=0, others=1).
    AtMinutesSeconds {
        /// Minutes value (0-59).
        minutes: u8,
        /// Seconds value (0-59).
        seconds: u8,
    },

    /// Trigger when hours, minutes, and seconds match (A1M1=0, A1M2=0, A1M3=0, A1M4=1)
    /// This creates a daily alarm at the specified time.
    AtTime {
        /// Hours value (0-23 for 24-hour, 1-12 for 12-hour).
        hours: u8,
        /// Minutes value (0-59).
        minutes: u8,
        /// Seconds value (0-59).
        seconds: u8,
        /// PM flag for 12-hour mode (None for 24-hour, Some(true/false) for 12-hour).
        is_pm: Option<bool>,
    },

    /// Trigger at specific time on specific date of month (all mask bits=0, DY/DT=0).
    AtTimeOnDate {
        /// Hours value (0-23 for 24-hour, 1-12 for 12-hour).
        hours: u8,
        /// Minutes value (0-59).
        minutes: u8,
        /// Seconds value (0-59).
        seconds: u8,
        /// Date of month (1-31).
        date: u8,
        /// PM flag for 12-hour mode (None for 24-hour, Some(true/false) for 12-hour).
        is_pm: Option<bool>,
    },

    /// Trigger at specific time on specific day of week (all mask bits=0, DY/DT=1).
    AtTimeOnDay {
        /// Hours value (0-23 for 24-hour, 1-12 for 12-hour).
        hours: u8,
        /// Minutes value (0-59).
        minutes: u8,
        /// Seconds value (0-59).
        seconds: u8,
        /// Day of week (1-7, where 1=Sunday).
        day: u8,
        /// PM flag for 12-hour mode (None for 24-hour, Some(true/false) for 12-hour).
        is_pm: Option<bool>,
    },
}

impl From<MyAlarm1Config> for Alarm1Config {
    fn from(value: MyAlarm1Config) -> Self {
        match value {
            MyAlarm1Config::EverySecond => Alarm1Config::EverySecond,
            MyAlarm1Config::AtSeconds { seconds } => Alarm1Config::AtSeconds { seconds },
            MyAlarm1Config::AtMinutesSeconds { minutes, seconds } => {
                Self::AtMinutesSeconds { minutes, seconds }
            }
            MyAlarm1Config::AtTime {
                hours,
                minutes,
                seconds,
                is_pm,
            } => Self::AtTime {
                hours,
                minutes,
                seconds,
                is_pm,
            },
            MyAlarm1Config::AtTimeOnDate {
                hours,
                minutes,
                seconds,
                date,
                is_pm,
            } => Self::AtTimeOnDate {
                hours,
                minutes,
                seconds,
                date,
                is_pm,
            },
            MyAlarm1Config::AtTimeOnDay {
                hours,
                minutes,
                seconds,
                day,
                is_pm,
            } => Self::AtTimeOnDay {
                hours,
                minutes,
                seconds,
                day,
                is_pm,
            },
        }
    }
}

async fn set_alarm_json(Json(json): Json<MyAlarm1Config>) -> impl IntoResponse {
    SET_ALARM.signal(json.into());
}

async fn get_clear_flags() -> impl IntoResponse {
    CLEAR_FLAGS_SIGNAL.signal(());
}
