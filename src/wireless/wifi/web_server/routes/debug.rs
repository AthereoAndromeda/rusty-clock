//! # Debug Routes
//! This contains routes only accessible during debug builds.

use ds3231::Alarm1Config;
use picoserve::{
    Router,
    routing::{PathRouter, get},
};

use crate::rtc_ds3231::SET_ALARM;

#[inline]
pub(super) fn add_routes(router: Router<impl PathRouter>) -> Router<impl PathRouter> {
    router.route("/debug/alarm", get(debug_alarm))
}

#[inline]
async fn debug_alarm() {
    let alarm_config = Alarm1Config::AtSeconds { seconds: 30 };
    SET_ALARM.signal(alarm_config);
}
