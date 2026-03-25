use picoserve::{
    Router,
    response::IntoResponse,
    routing::{PathRouter, get},
};

use crate::lcd::{LCD_COMMANDS, LcdAction};

#[inline]
pub(super) fn add_routes(router: Router<impl PathRouter>) -> Router<impl PathRouter> {
    let router = router.route("/lcd/on", get(backlight_control_on));
    let router = router.route("/lcd/off", get(backlight_control_off));
    router.route("/lcd/toggle", get(backlight_control_toggle))
}

#[inline]
async fn backlight_control_on() -> impl IntoResponse {
    LCD_COMMANDS.signal(LcdAction::BacklightOn);
}
#[inline]
async fn backlight_control_off() -> impl IntoResponse {
    LCD_COMMANDS.signal(LcdAction::BacklightOff);
}
#[inline]
async fn backlight_control_toggle() -> impl IntoResponse {
    LCD_COMMANDS.signal(LcdAction::BacklightToggle);
}
