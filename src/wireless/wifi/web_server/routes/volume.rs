use embassy_time::Timer;
use picoserve::{
    Router,
    extract::Form,
    response::IntoResponse,
    routing::{PathRouter, post},
};

use crate::buzzer::{BUZZER_ACTION_SIGNAL, VOLUME_SIGNAL};

#[derive(serde::Deserialize)]
struct VolumeForm {
    volume: u8,
}

pub(super) fn add_routes(router: Router<impl PathRouter>) -> Router<impl PathRouter> {
    router.route("/volume", post(post_volume))
}

async fn post_volume(Form(form): Form<VolumeForm>) -> impl IntoResponse {
    VOLUME_SIGNAL.signal(form.volume);
    Timer::after_secs(1).await;
    BUZZER_ACTION_SIGNAL.signal(crate::buzzer::BuzzerAction::On);
}
