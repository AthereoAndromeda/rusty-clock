use crate::buzzer::{BUZZER_ACTION_SIGNAL, BuzzerAction, IS_BUZZER_ON};
use embassy_time::Timer;
use picoserve::{
    Router,
    extract::Form,
    response::{DebugValue, IntoResponse, StatusCode},
    routing::{PathRouter, get, post},
};

#[derive(serde::Deserialize)]
struct VolumeForm {
    volume: u8,
}

struct BuzzerEvent;

impl picoserve::response::sse::EventSource for BuzzerEvent {
    async fn write_events<W: picoserve::io::Write>(
        self,
        mut writer: picoserve::response::sse::EventWriter<'_, W>,
    ) -> Result<(), W::Error> {
        loop {
            let a = IS_BUZZER_ON.load(core::sync::atomic::Ordering::Acquire);
            let res = if a { "true" } else { "false" };

            writer.write_event("", res).await?;
            Timer::after_secs(1).await;
        }
    }
}

#[inline]
pub(super) fn add_routes(router: Router<impl PathRouter>) -> Router<impl PathRouter> {
    router
        .route("/buzzer", get(get_buzzer))
        .route("/buzzer/toggle", get(toggle_buzzer))
        .route("/buzzer/on", get(toggle_buzzer_on))
        .route("/buzzer/off", get(toggle_buzzer_off))
        .route(
            "/buzzer/stream",
            get(async || picoserve::response::EventStream(BuzzerEvent)),
        )
        .route("/volume", post(post_volume))
}

#[inline]
async fn get_buzzer() -> impl IntoResponse {
    let state = IS_BUZZER_ON.load(core::sync::atomic::Ordering::Acquire);
    DebugValue(state)
}

#[inline]
async fn toggle_buzzer() -> impl IntoResponse {
    BUZZER_ACTION_SIGNAL.signal(BuzzerAction::Toggle);
}
#[inline]
async fn toggle_buzzer_on() -> impl IntoResponse {
    BUZZER_ACTION_SIGNAL.signal(BuzzerAction::On);
}
#[inline]
async fn toggle_buzzer_off() -> impl IntoResponse {
    BUZZER_ACTION_SIGNAL.signal(BuzzerAction::Off);
}

#[inline]
async fn post_volume(Form(form): Form<VolumeForm>) -> impl IntoResponse {
    use crate::buzzer::BuzzerAction;

    if form.volume > 100 {
        return Err(StatusCode::BAD_REQUEST);
    }

    BUZZER_ACTION_SIGNAL.signal(BuzzerAction::SetVolume(form.volume));
    #[cfg(debug_assertions)]
    {
        // To see effect of volume while debugging.
        // Should not automatically turn on buzzer at production
        embassy_time::Timer::after_millis(300).await;
        BUZZER_ACTION_SIGNAL.signal(BuzzerAction::On);
    }
    Ok(())
}
