mod routes;
use routes::*;

use defmt::info;
use embassy_time::Duration;
use picoserve::{
    AppBuilder, AppRouter, Router, make_static,
    response::File,
    routing::{get, get_service, parse_path_segment, post},
};

#[cfg(debug_assertions)]
const HTML_STR: &str = include_str!("./../../../../resources/index.html");
const HTML_MIN_BR: &[u8] = include_bytes!("./../../../../resources/index.min.html.br");

pub const WEB_TASK_POOL_SIZE: usize = 3;

/// Our Web server App
pub struct App;

impl AppBuilder for App {
    type PathRouter = impl picoserve::routing::PathRouter;

    fn build_app(self) -> picoserve::Router<Self::PathRouter> {
        let router = Router::new()
            // WARN: HTMX CDN is used instead of being bundled. This means if
            // client is not connected to the internet, webpage will not work
            //
            // WARN: Firefox does not support brotli on HTTP by default.
            // Go to about:config and add br to `network.http.accept-encoding`
            //
            // TODO?: Bundle HTMX with page? (has to be compressed beforehand)
            // to keep binary size small
            .route(
                "/",
                get_service(File::with_content_type_and_headers(
                    "text/html",
                    HTML_MIN_BR,
                    &[("content-encoding", "br")],
                )),
            )
            .route("/help", get(get_help))
            .route("/time", get(get_time))
            .route("/epoch", get(get_epoch))
            .route("/alarm", get(get_alarm))
            .route("/alarm_submit", post(set_alarm_form))
            .route(
                (
                    "/alarm",
                    parse_path_segment::<u8>(),
                    parse_path_segment::<u8>(),
                    parse_path_segment::<u8>(),
                ),
                get(set_alarm),
            )
            .route("/buzzer", get(get_buzzer))
            .route("/buzzer/toggle", get(toggle_buzzer))
            .route("/buzzer/on", get(toggle_buzzer_on))
            .route("/buzzer/off", get(toggle_buzzer_off))
            .route(("/timer", parse_path_segment::<i32>()), get(set_timer))
            .route("/sync", get(get_sync))
            .route(
                "/events",
                get(async || picoserve::response::EventStream(TimeEvent)),
            );

        #[cfg(debug_assertions)]
        let router = router.route("/", get_service(File::html(HTML_STR)));

        router
    }
}

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

    const PORT: u16 = {
        let s = option_env!("WEB_PORT").unwrap_or("80");
        // SAFETY: User must ensure WEB_PORT is a valid number
        unsafe { u16::from_str_radix(s, 10).unwrap_unchecked() }
    };

    stack.wait_config_up().await;
    let addr = stack.config_v4().unwrap().address;

    info!(
        "[task-id:{}] Serving and listening at {}:{}",
        task_id, addr, PORT
    );
    picoserve::Server::new(app, config, &mut http_buffer)
        .listen_and_serve(task_id, stack, PORT, &mut tcp_rx_buffer, &mut tcp_tx_buffer)
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
            persistent_start_read_request: Some(Duration::from_secs(2)),
            read_request: Some(Duration::from_secs(2)),
            write: Some(Duration::from_secs(5)),
        })
        .keep_connection_alive()
    );

    (app, config)
}
