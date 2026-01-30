use crate::buzzer::{BUZZER_SIGNAL, BuzzerAction, IS_BUZZER_ON};
use picoserve::{
    Router,
    response::{DebugValue, IntoResponse},
    routing::{PathRouter, get},
};

pub fn add_routes(router: Router<impl PathRouter>) -> Router<impl PathRouter> {
    router
        .route("/buzzer", get(get_buzzer))
        .route("/buzzer/toggle", get(toggle_buzzer))
        .route("/buzzer/on", get(toggle_buzzer_on))
        .route("/buzzer/off", get(toggle_buzzer_off))
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
