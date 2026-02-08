pub(super) mod alarm;
pub(super) mod buzzer;
pub(super) mod time;
pub(super) mod timer;

pub(super) async fn get_help() -> &'static str {
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
