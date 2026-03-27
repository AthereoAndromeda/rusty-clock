use picoserve::{
    Router,
    extract::Form,
    response::IntoResponse,
    routing::{PathRouter, get},
};
use serde::Deserialize;

use crate::lcd::{LCD_COMMANDS, LcdAction, LcdDisplayString};

#[inline]
pub(super) fn add_routes(router: Router<impl PathRouter>) -> Router<impl PathRouter> {
    router
        .route("/lcd/on", get(backlight_control_on))
        .route("/lcd/off", get(backlight_control_off))
        .route("/lcd/display", get(lcd_display))
        .route("/lcd/toggle", get(backlight_control_toggle))
}

#[derive(Deserialize)]
struct DisplayForm {
    s1: LcdDisplayString,
    s2: Option<LcdDisplayString>,
}

#[inline]
async fn lcd_display(Form(form): Form<DisplayForm>) {
    if let Some(s2) = form.s2 {
        LCD_COMMANDS.signal(LcdAction::DisplayLines(form.s1, s2));
    } else {
        LCD_COMMANDS.signal(LcdAction::Display(form.s1));
    }
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
