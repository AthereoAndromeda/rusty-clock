use chrono::FixedOffset;
use embassy_time::Timer;
use picoserve::{
    Router,
    extract::Query,
    response::{DebugValue, IntoResponse},
    routing::{PathRouter, get},
};

use crate::{TZ_OFFSET, rtc_ds3231::TIME_WATCH, wireless::wifi::sntp::NTP_SYNC};

#[derive(Debug, serde::Deserialize)]
struct TimeQueryParams {
    pub utc: Option<bool>,
}

struct TimeEvent;

impl picoserve::response::sse::EventSource for TimeEvent {
    async fn write_events<W: picoserve::io::Write>(
        self,
        mut writer: picoserve::response::sse::EventWriter<'_, W>,
    ) -> Result<(), W::Error> {
        let mut anon_recv = TIME_WATCH.anon_receiver();

        loop {
            #[cfg(debug_assertions)]
            defmt::debug!("[sse:time] Writing Event...");

            let time = anon_recv.try_get();
            if time.is_none() {
                writer.write_keepalive().await?;
                Timer::after_secs(1).await;
                continue;
            }

            writer.write_event("", time.unwrap()).await?;

            #[cfg(debug_assertions)]
            defmt::debug!("[sse:time] Event Written!");
            Timer::after_secs(1).await;
        }
    }
}

pub(super) fn add_routes(router: Router<impl PathRouter>) -> Router<impl PathRouter> {
    router
        .route("/time", get(get_time))
        .route("/epoch", get(get_epoch))
        .route("/sync", get(get_sync))
        .route(
            "/time/stream",
            get(async || picoserve::response::EventStream(TimeEvent)),
        )
}

async fn get_epoch() -> impl IntoResponse {
    let rtc_time = TIME_WATCH.receiver().expect("Maximum reached").get().await;
    let epoch = rtc_time.and_utc().timestamp();

    DebugValue(epoch)
}

async fn get_time(Query(query): Query<TimeQueryParams>) -> impl IntoResponse {
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

async fn get_sync() -> impl IntoResponse {
    NTP_SYNC.signal(());
}
