use picoserve::{
    Router,
    response::IntoResponse,
    routing::{PathRouter, get},
};

use crate::lcd::{BACKLIGHT_SIGNAL, LcdAction};

pub(super) fn add_routes(router: Router<impl PathRouter>) -> Router<impl PathRouter> {
    let router = router.route("/lcd/on", get(backlight_control_on));
    let router = router.route("/lcd/off", get(backlight_control_off));
    router.route("/lcd/toggle", get(backlight_control_toggle))
}

async fn backlight_control_on() -> impl IntoResponse {
    BACKLIGHT_SIGNAL.signal(LcdAction::BacklightOn);
}
async fn backlight_control_off() -> impl IntoResponse {
    BACKLIGHT_SIGNAL.signal(LcdAction::BacklightOff);
}
async fn backlight_control_toggle() -> impl IntoResponse {
    BACKLIGHT_SIGNAL.signal(LcdAction::BacklightToggle);
}
