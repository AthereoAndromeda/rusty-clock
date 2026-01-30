use chrono::FixedOffset;
use picoserve::{
    Router,
    extract::Query,
    response::{DebugValue, IntoResponse},
    routing::{PathRouter, get},
};

use crate::{TZ_OFFSET, rtc_ds3231::TIME_WATCH};

pub fn add_routes(router: Router<impl PathRouter>) -> Router<impl PathRouter> {
    router
        .route("/time", get(get_time))
        .route("/epoch", get(get_epoch))
}

async fn get_epoch() -> impl IntoResponse {
    let rtc_time = TIME_WATCH.receiver().expect("Maximum reached").get().await;
    let epoch = rtc_time.and_utc().timestamp();

    DebugValue(epoch)
}

async fn get_time(Query(query): Query<super::alarm::AlarmQueryParams>) -> impl IntoResponse {
    let is_utc = query.utc.is_some_and(|p| p);
    let time = TIME_WATCH.receiver().expect("Maximum reached").get().await;

    let res = if is_utc {
        time
    } else {
        let offset = FixedOffset::east_opt((TZ_OFFSET as i32) * 3600).unwrap();
        let a = time.and_utc().with_timezone(&offset).naive_local();
        a.into()
    };

    DebugValue(res.to_human_local())
}
