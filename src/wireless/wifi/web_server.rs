use embassy_time::Duration;
use picoserve::{
    AppBuilder, AppRouter, Router, make_static,
    response::{DebugValue, IntoResponse},
    routing::get,
};

use crate::EPOCH_SIGNAL;

async fn root() -> &'static str {
    r#"
Hello from ESP32! This is the web server for rusty clock

Paths: 
/                         - Prints this help message.
/time                     - Gets current time
/alarm                    - Gets alarm settings
/alarm/:hour/:minute      - Sets alarm
"#
}

async fn get_time() -> impl IntoResponse {
    let time = EPOCH_SIGNAL.wait().await;
    DebugValue(time)
}

/// Our Web server App
pub struct App;

impl AppBuilder for App {
    type PathRouter = impl picoserve::routing::PathRouter;

    fn build_app(self) -> picoserve::Router<Self::PathRouter> {
        Router::new()
            .route("/", get(root))
            .route("/time", get(get_time))
    }
}

pub const WEB_TASK_POOL_SIZE: usize = 3;

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
