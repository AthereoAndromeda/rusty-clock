//! # Web Server
//! This module contains the logic for hosting a web server.
//!
//! Currently this sends a web page which is used as a control
//! panel to remotely control and configure the device.

mod routes;
use defmt::info;
use embassy_time::Duration;
use picoserve::{AppBuilder, AppRouter, Router, make_static, response::File, routing::get_service};
use static_cell::ConstStaticCell;

pub(super) const WEB_TASK_POOL_SIZE: usize = 3;

/// Our Web server App
pub(super) struct App;

impl AppBuilder for App {
    type PathRouter = impl picoserve::routing::PathRouter;

    fn build_app(self) -> picoserve::Router<Self::PathRouter> {
        // WARN: HTMX CDN is used instead of being bundled. This means if
        // client is not connected to the internet, webpage will not work
        //
        // WARN: Firefox does not support brotli on HTTP by default.
        // Go to about:config and add br to `network.http.accept-encoding`
        //
        // TODO?: Bundle HTMX with page? (has to be compressed beforehand)
        // to keep binary size small
        let router = routes::add_all_routes(Router::new());

        // Use unminified when debug, minified when release build
        if cfg!(debug_assertions) {
            router.route(
                "/",
                get_service(File::with_content_type(
                    "text/html",
                    include_bytes!("./../../../../resources/index.html"),
                )),
            )
        } else {
            router.route(
                "/",
                get_service(File::with_content_type_and_headers(
                    "text/html",
                    include_bytes!("./../../../../resources/index.min.html.br"),
                    &[("content-encoding", "br")],
                )),
            )
        }
    }
}

// This is used since simply using `static` inside the function
// will cause a runtime panic since the cell would be taken twice.
//
// This method ensures that each buffer has a unique memory address
static RX_BUFFERS: [ConstStaticCell<[u8; 1024]>; WEB_TASK_POOL_SIZE] =
    [const { ConstStaticCell::new([0; _]) }; WEB_TASK_POOL_SIZE];
static TX_BUFFERS: [ConstStaticCell<[u8; 1024]>; WEB_TASK_POOL_SIZE] =
    [const { ConstStaticCell::new([0; _]) }; WEB_TASK_POOL_SIZE];
static HTTP_BUFFERS: [ConstStaticCell<[u8; 2048]>; WEB_TASK_POOL_SIZE] =
    [const { ConstStaticCell::new([0; _]) }; WEB_TASK_POOL_SIZE];

#[embassy_executor::task(pool_size = WEB_TASK_POOL_SIZE)]
pub(super) async fn web_task(
    task_id: usize,
    stack: embassy_net::Stack<'static>,
    app: &'static AppRouter<App>,
    config: &'static picoserve::Config,
) -> ! {
    let tcp_rx_buffer = RX_BUFFERS[task_id].take();
    let tcp_tx_buffer = TX_BUFFERS[task_id].take();
    let http_buffer = HTTP_BUFFERS[task_id].take();

    const PORT: u16 = {
        let s = option_env!("WEB_PORT").unwrap_or("80");

        u16::from_str_radix(s, 10)
            .ok()
            .expect("Failed to parse .env: WEB_PORT")
    };

    stack.wait_config_up().await;
    let addr = stack.config_v4().unwrap().address;

    info!(
        "[task-id:{}] Serving and listening at {}:{}",
        task_id, addr, PORT
    );
    picoserve::Server::new(app, config, http_buffer)
        .listen_and_serve(task_id, stack, PORT, tcp_rx_buffer, tcp_tx_buffer)
        .await
        .into_never()
}

pub(super) fn init() -> (
    &'static mut Router<<App as AppBuilder>::PathRouter>,
    &'static mut picoserve::Config,
) {
    let app = make_static!(AppRouter<App>, App.build_app());
    let config = make_static!(
        picoserve::Config,
        picoserve::Config::new(picoserve::Timeouts {
            start_read_request: Duration::from_secs(5),
            persistent_start_read_request: Duration::from_secs(2),
            read_request: Duration::from_secs(2),
            write: Duration::from_secs(5),
        })
        .keep_connection_alive()
    );

    (app, config)
}
