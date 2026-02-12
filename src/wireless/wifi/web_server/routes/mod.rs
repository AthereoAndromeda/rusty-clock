use picoserve::{
    Router,
    routing::{PathRouter, get},
};

mod alarm;
mod buzzer;
mod time;
mod timer;

// TODO: Dynamically generate help message
// Attribute macro? #[help_msg = "message"]
async fn get_help() -> &'static str {
    r#"
Hello from ESP32! This is the web server for rusty clock

All paths use GET unless otherwise specified

Paths:
/                         - Gets Control Panel webpage
/help                     - Prints this help message.
/time                     - Gets current time
/epoch                    - Gets current time as UNIX_EPOCH

/alarm                    - Gets alarm settings
/alarm/:hour/:minute      - Sets alarm
/alarm/off                - Turns off alarm if active
/alarm/on
/alarm/toggle                

/timer                    - Set a timer to buzz
"#
}

pub(super) fn add_all_routes(router: Router<impl PathRouter>) -> Router<impl PathRouter> {
    let router = alarm::add_routes(router);
    let router = buzzer::add_routes(router);
    let router = time::add_routes(router);
    let router = timer::add_routes(router);

    router.route("/help", get(get_help))
}
