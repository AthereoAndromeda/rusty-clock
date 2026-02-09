use crate::buzzer::{BUZZER_SIGNAL, BuzzerAction, IS_BUZZER_ON};
use embassy_time::Timer;
use picoserve::{
    Router,
    response::{DebugValue, IntoResponse},
    routing::{PathRouter, get},
};

struct BuzzerEvent;

impl picoserve::response::sse::EventSource for BuzzerEvent {
    async fn write_events<W: picoserve::io::Write>(
        self,
        mut writer: picoserve::response::sse::EventWriter<'_, W>,
    ) -> Result<(), W::Error> {
        loop {
            let a = IS_BUZZER_ON.load(core::sync::atomic::Ordering::SeqCst);
            let res = if a { "true" } else { "false" };

            writer.write_event("", res).await?;
            Timer::after_secs(1).await;
        }
    }
}

pub fn add_routes(router: Router<impl PathRouter>) -> Router<impl PathRouter> {
    router
        .route("/buzzer", get(get_buzzer))
        .route("/buzzer/toggle", get(toggle_buzzer))
        .route("/buzzer/on", get(toggle_buzzer_on))
        .route("/buzzer/off", get(toggle_buzzer_off))
        .route(
            "/buzzer/stream",
            get(async || picoserve::response::EventStream(BuzzerEvent)),
        )
}

async fn get_buzzer() -> impl IntoResponse {
    let state = IS_BUZZER_ON.load(core::sync::atomic::Ordering::SeqCst);
    DebugValue(state)
}

async fn toggle_buzzer() -> impl IntoResponse {
    BUZZER_SIGNAL.signal(BuzzerAction::Toggle);
}
async fn toggle_buzzer_on() -> impl IntoResponse {
    BUZZER_SIGNAL.signal(BuzzerAction::On);
}
async fn toggle_buzzer_off() -> impl IntoResponse {
    BUZZER_SIGNAL.signal(BuzzerAction::Off);
}
