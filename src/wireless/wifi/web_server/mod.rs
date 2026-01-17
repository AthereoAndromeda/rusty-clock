mod routes;
use routes::*;

use defmt::info;
use embassy_time::Duration;
use picoserve::{
    AppBuilder, AppRouter, Router, make_static,
    routing::{get, parse_path_segment},
};
/// Our Web server App
pub struct App;

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
                    parse_path_segment::<u8>(),
                ),
                get(set_alarm),
            )
            .route("/buzzer/toggle", get(toggle_buzzer))
            .route("/buzzer/on", get(toggle_buzzer_on))
            .route("/buzzer/off", get(toggle_buzzer_off))
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
    let mut tcp_rx_buffer = [0; 1024];
    let mut tcp_tx_buffer = [0; 1024];
    let mut http_buffer = [0; 2048];

    let port = option_env!("WEB_PORT")
        .unwrap_or("80")
        .parse::<u16>()
        .unwrap();

    stack.wait_config_up().await;
    let addr = stack.config_v4().unwrap().address;

    info!("Serving and listening at {}:{}", addr, port);
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
