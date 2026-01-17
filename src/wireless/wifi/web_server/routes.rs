use chrono::FixedOffset;
use ds3231::Alarm1Config;
use picoserve::{
    extract::Query,
    response::{DebugValue, IntoResponse},
};
use serde::Deserialize;

use crate::{
    TIME_SIGNAL, TZ_OFFSET,
    buzzer::{BUZZER_SIGNAL, BuzzerState, TIMER_SIGNAL},
    rtc_ds3231::{ALARM_REQUEST, ALARM_SIGNAL, SET_ALARM},
};

pub(super) async fn root() -> &'static str {
    r#"
Hello from ESP32! This is the web server for rusty clock

All paths use GET unless otherwise specified

Paths: 
/                         - Prints this help message.
/time                     - Gets current time
/epoch                    - Gets current time as UNIX_EPOCH

/alarm                    - Gets alarm settings
/alarm/:hour/:minute      - Sets alarm
/alarm/off                - Turns off alarm if active
/alarm/on
/alarm/toggle                

/timer                    - Set a timer to buzz
"#
}

pub(super) async fn get_epoch() -> impl IntoResponse {
    let rtc_time = &TIME_SIGNAL.wait().await;
    let epoch = rtc_time.and_utc().timestamp();

    DebugValue(epoch)
}

pub(super) async fn get_time(Query(query): Query<AlarmQueryParams>) -> impl IntoResponse {
    let is_utc = query.utc.is_some_and(|p| p);
    let time = TIME_SIGNAL.wait().await;

    let res = if is_utc {
        time
    } else {
        let offset = FixedOffset::east_opt((*TZ_OFFSET.get() as i32) * 3600).unwrap();
        let a = time.and_utc().with_timezone(&offset).naive_local();
        a.into()
    };

    DebugValue(res)
}

pub(super) async fn get_alarm() -> impl IntoResponse {
    ALARM_REQUEST.signal(true); // Send anything to trigger
    let response = ALARM_SIGNAL.wait().await;
    ALARM_REQUEST.reset();

    DebugValue(response)
}

pub(super) async fn set_alarm(
    (hour, minute, sec): (u8, u8, u8),
    Query(query): Query<AlarmQueryParams>,
) -> impl IntoResponse {
    let base_time = jiff::civil::time(hour as i8, minute as i8, sec as i8, 0);

    let time = if query.utc.is_some_and(|p| p) {
        base_time
    } else {
        base_time.wrapping_sub(jiff::Span::new().hours(*TZ_OFFSET.get()))
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

    "Alarm Set!"
}

pub(super) async fn set_timer(sec: i32) -> impl IntoResponse {
    TIMER_SIGNAL.signal(sec);
}

pub(super) async fn toggle_buzzer() -> impl IntoResponse {
    BUZZER_SIGNAL.signal(BuzzerState::Toggle);
}
pub(super) async fn toggle_buzzer_on() -> impl IntoResponse {
    BUZZER_SIGNAL.signal(BuzzerState::On);
}
pub(super) async fn toggle_buzzer_off() -> impl IntoResponse {
    BUZZER_SIGNAL.signal(BuzzerState::Off);
}

#[derive(Debug, Deserialize)]
pub(super) struct AlarmQueryParams {
    utc: Option<bool>,
}
