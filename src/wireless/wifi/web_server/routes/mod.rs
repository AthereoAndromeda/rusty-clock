//! # Web Server Routes
//! Holds all the routes for the web server.

use picoserve::{
    Router,
    routing::{PathRouter, get},
};

mod alarm;
mod buzzer;
#[cfg(debug_assertions)]
mod debug;
mod lcd;
mod time;
mod timer;

macro_rules! add_routes {
    ($router:ident; $($name:ident),* $(,)?) => {{
        let router = $router;
        $(let router = $name::add_routes(router);)*
        router
    }};
}

#[inline]
pub(super) fn add_all_routes(router: Router<impl PathRouter>) -> Router<impl PathRouter> {
    let router = add_routes!(
        router;
        alarm,
        buzzer,
        time,
        timer,
        lcd,
    );

    #[cfg(debug_assertions)]
    let router = debug::add_routes(router);
    router.route("/help", get(get_help))
}

#[inline]
// TODO: Dynamically generate help message
// Attribute macro? #[help_msg = "message"]
async fn get_help() -> &'static str {
    "Hello from ESP32! This is the web server for rusty clock

Paths:
GET /                         - Gets Control Panel webpage
GET /help                     - Prints this help message.

GET /time                     - Gets current time
GET /epoch                    - Gets current time as UNIX_EPOCH
GET /uptime                   - Gets uptime of MCU
GET /sync                     - Syncs RTC time with NTP
SSE /time/stream

GET /alarm                    - Gets alarm settings
GET /alarm/clear              - Clear RTC Flags
GET /alarm/:hour/:min/:sec    - Sets alarm
GET /alarm/off                - Turns off alarm if active
GET /alarm/on
GET /alarm/toggle
POST /alarm/submit 

GET /buzzer                   - Gets current buzzer volume
GET /buzzer/toggle
GET /buzzer/on
GET /buzzer/off
POST /volume
SSE /buzzer/stream

GET /lcd/on
GET /lcd/off
GET /lcd/toggle
POST /lcd/display

GET /timer/:sec               - Set timer in seconds
POST /timer
"
}
