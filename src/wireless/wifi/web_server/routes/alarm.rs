use chrono::Timelike;
use ds3231::Alarm1Config;
use picoserve::{
    Router,
    extract::{Form, Query},
    response::{DebugValue, IntoResponse},
    routing::{PathRouter, get, parse_path_segment, post},
};
use serde::Deserialize;

use crate::{
    TZ_OFFSET,
    rtc_ds3231::{ALARM_CONFIG_RWLOCK, CLEAR_FLAGS_SIGNAL, SET_ALARM},
};

pub fn add_routes(router: Router<impl PathRouter>) -> Router<impl PathRouter> {
    router
        .route("/alarm", get(get_alarm))
        .route("/alarm/clear", get(get_clear_flags))
        .route("/alarm_submit", post(set_alarm_form))
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
pub struct AlarmQueryParams {
    pub utc: Option<bool>,
}

async fn get_alarm() -> impl IntoResponse {
    let response = ALARM_CONFIG_RWLOCK.read().await;
    DebugValue(response)
}

async fn set_alarm_inner(hour: u8, min: u8, sec: u8, is_utc: bool) {
    let base_time = chrono::NaiveTime::from_hms_opt(hour as u32, min as u32, sec as u32).unwrap();

    let time = if is_utc {
        base_time
    } else {
        base_time
            .overflowing_sub_signed(chrono::TimeDelta::hours(TZ_OFFSET as i64))
            .0
    };

    #[cfg(debug_assertions)]
    defmt::debug!("{} {} {}", time.hour(), time.minute(), time.second());

    let conf = Alarm1Config::AtTime {
        hours: time.hour() as u8,
        minutes: time.minute() as u8,
        seconds: time.second() as u8,
        is_pm: None,
    };

    SET_ALARM.signal(conf);
}

async fn set_alarm(
    (hour, min, sec): (u8, u8, u8),
    Query(query): Query<AlarmQueryParams>,
) -> impl IntoResponse {
    set_alarm_inner(hour, min, sec, query.utc.is_some_and(|x| x)).await;
    "Alarm Set!"
}

#[derive(Debug, Deserialize)]
struct AlarmForm {
    pub hour: u8,
    pub min: u8,
    pub sec: u8,
    pub is_utc: heapless::String<3>,
}

async fn set_alarm_form(Form(form): Form<AlarmForm>) -> impl IntoResponse {
    defmt::debug!("{}", defmt::Debug2Format(&form));
    let AlarmForm {
        hour,
        min,
        sec,
        is_utc,
    } = form;

    set_alarm_inner(hour, min, sec, is_utc == "on").await
}

async fn get_clear_flags() -> impl IntoResponse {
    CLEAR_FLAGS_SIGNAL.signal(());
}
