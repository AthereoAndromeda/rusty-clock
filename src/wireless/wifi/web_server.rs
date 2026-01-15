use ds3231::Alarm1Config;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::Duration;
use picoserve::{
    AppBuilder, AppRouter, Router,
    extract::Query,
    make_static,
    response::{DebugValue, IntoResponse},
    routing::{get, parse_path_segment},
};
use serde::Deserialize;

use crate::{EPOCH_SIGNAL, TIME_SIGNAL};

pub static ALARM_REQUEST: Signal<CriticalSectionRawMutex, bool> = Signal::new();
pub static ALARM_SIGNAL: Signal<CriticalSectionRawMutex, Alarm1Config> = Signal::new();
pub static SET_ALARM: Signal<CriticalSectionRawMutex, Alarm1Config> = Signal::new();

async fn root() -> &'static str {
    r#"
Hello from ESP32! This is the web server for rusty clock

All paths use GET unless otherwise specified

Paths: 
/                         - Prints this help message.
/time                     - Gets current time
/epoch                    - Gets current time as UNIX_EPOCH
/alarm                    - Gets alarm settings
/alarm/:hour/:minute      - Sets alarm
/timer                    - Set a timer to buzz
"#
}

async fn get_epoch() -> impl IntoResponse {
    let time = EPOCH_SIGNAL.wait().await;
    DebugValue(time)
}

async fn get_time() -> impl IntoResponse {
    let time = TIME_SIGNAL.wait().await;
    DebugValue(time)
}

async fn get_alarm() -> impl IntoResponse {
    ALARM_REQUEST.signal(true); // Send anything to trigger
    let response = ALARM_SIGNAL.wait().await;
    ALARM_REQUEST.reset();

    DebugValue(response)
}

async fn set_alarm(
    (hour, minute): (u8, u8),
    Query(query): Query<AlarmQueryParams>,
) -> impl IntoResponse {
    let conf = Alarm1Config::AtTime {
        hours: hour,
        minutes: minute,
        seconds: 0,
        is_pm: None,
    };

    SET_ALARM.signal(conf);

    "Alarm Set!"
}

async fn set_timer(sec: i32) -> impl IntoResponse {
    // TIMER_SIGNAL
}

/// Our Web server App
pub struct App;

#[derive(Debug, Deserialize)]
struct AlarmQueryParams {
    hour: Option<i32>,
    min: Option<i32>,
    sec: Option<i32>,
}

impl AppBuilder for App {
    type PathRouter = impl picoserve::routing::PathRouter;

    fn build_app(self) -> picoserve::Router<Self::PathRouter> {
        Router::new()
            .route("/", get(root))
            .route("/time", get(get_time))
            .route("/epoch", get(get_epoch))
            .route("/alarm", get(get_alarm))
            .route(
                (
                    "/alarm",
                    parse_path_segment::<u8>(),
                    parse_path_segment::<u8>(),
                ),
                get(set_alarm),
            )
            .route(("/timer", parse_path_segment::<i32>()), get(set_timer))
    }
}

pub const WEB_TASK_POOL_SIZE: usize = 2;

#[embassy_executor::task(pool_size = WEB_TASK_POOL_SIZE)]
pub async fn web_task(
    task_id: usize,
    stack: embassy_net::Stack<'static>,
    app: &'static AppRouter<App>,
    config: &'static picoserve::Config<Duration>,
) -> ! {
    let port = 80;
    let mut tcp_rx_buffer = [0; 1024];
    let mut tcp_tx_buffer = [0; 1024];
    let mut http_buffer = [0; 2048];

    picoserve::Server::new(app, config, &mut http_buffer)
        .listen_and_serve(task_id, stack, port, &mut tcp_rx_buffer, &mut tcp_tx_buffer)
        .await
        .into_never()
}

pub fn init_web() -> (
    &'static mut Router<<App as AppBuilder>::PathRouter>,
    &'static mut picoserve::Config<embassy_time::Duration>,
) {
    let app = make_static!(AppRouter<App>, App.build_app());
    let config = make_static!(
        picoserve::Config<Duration>,
        picoserve::Config::new(picoserve::Timeouts {
            start_read_request: Some(Duration::from_secs(5)),
            persistent_start_read_request: Some(Duration::from_secs(1)),
            read_request: Some(Duration::from_secs(1)),
            write: Some(Duration::from_secs(1)),
        })
        .keep_connection_alive()
    );

    (app, config)
}
