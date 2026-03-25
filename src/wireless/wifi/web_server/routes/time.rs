use embassy_time::Timer;
use picoserve::{
    Router,
    extract::Query,
    response::{DebugValue, IntoResponse},
    routing::{PathRouter, get},
};

use crate::{BOOT_TIME, rtc_ds3231::TIME_WATCH, wireless::wifi::sntp::NTP_SYNC};

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
            defmt::trace!("[sse:time] Writing Event...");

            let time = anon_recv.try_get();
            if time.is_none() {
                writer.write_keepalive().await?;
                Timer::after_secs(1).await;
                continue;
            }

            writer.write_event("", time.unwrap()).await?;

            #[cfg(debug_assertions)]
            defmt::trace!("[sse:time] Event Written!");
            Timer::after_secs(1).await;
        }
    }
}

#[inline]
pub(super) fn add_routes(router: Router<impl PathRouter>) -> Router<impl PathRouter> {
    router
        .route("/time", get(get_time))
        .route("/epoch", get(get_epoch))
        .route("/sync", get(get_sync))
        .route("/uptime", get(get_uptime))
        .route(
            "/time/stream",
            get(async || picoserve::response::EventStream(TimeEvent)),
        )
}

#[inline]
async fn get_uptime() -> impl IntoResponse {
    DebugValue(BOOT_TIME.elapsed().as_minutes())
}

#[inline]
async fn get_epoch() -> impl IntoResponse {
    let rtc_time = TIME_WATCH.receiver().expect("Maximum reached").get().await;
    let epoch = rtc_time.to_utc().timestamp();
    DebugValue(epoch)
}

#[inline]
async fn get_time(Query(query): Query<TimeQueryParams>) -> impl IntoResponse {
    use embassy_futures::select::Either;
    let is_utc = query.utc.is_some_and(|p| p);
    let time = TIME_WATCH.receiver().expect("Maximum reached").get().await;

    let res = if is_utc {
        Either::First(time)
    } else {
        // let datetime: RtcDateTime<chrono::FixedOffset> =
        //     time.to_utc().with_timezone(&FIXED_OFFSET).into();
        let datetime = time.local();
        Either::Second(datetime)
    };

    match res {
        Either::First(rtc) => DebugValue(rtc.to_human()),
        Either::Second(rtc) => DebugValue(rtc.to_human()),
    }
}

#[inline]
async fn get_sync() -> impl IntoResponse {
    NTP_SYNC.signal(());
}
