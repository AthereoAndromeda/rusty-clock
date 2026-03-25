use picoserve::{
    Router,
    extract::Form,
    response::{IntoResponse, StatusCode},
    routing::{PathRouter, post},
};

use crate::buzzer::BUZZER_ACTION_SIGNAL;

#[derive(serde::Deserialize)]
struct VolumeForm {
    volume: u8,
}

#[inline]
pub(super) fn add_routes(router: Router<impl PathRouter>) -> Router<impl PathRouter> {
    router.route("/volume", post(post_volume))
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
