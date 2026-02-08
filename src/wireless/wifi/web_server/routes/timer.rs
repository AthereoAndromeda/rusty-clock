use picoserve::{
    extract::Form,
    response::IntoResponse,
    routing::{get, parse_path_segment, post},
};

use crate::{buzzer::TIMER_SIGNAL, wireless::wifi::web_server::routes::AddRoute};

pub struct Route;

impl AddRoute for Route {
    fn add_routes(
        router: picoserve::Router<impl picoserve::routing::PathRouter>,
    ) -> picoserve::Router<impl picoserve::routing::PathRouter> {
        router
            .route("/timer", post(timer_form))
            .route(("/timer", parse_path_segment::<u32>()), get(set_timer))
    }
}
pub fn add_routes(
    router: picoserve::Router<impl picoserve::routing::PathRouter>,
) -> picoserve::Router<impl picoserve::routing::PathRouter> {
    router
        .route("/timer", post(timer_form))
        .route(("/timer", parse_path_segment::<u32>()), get(set_timer))
}

#[derive(Debug, serde::Deserialize)]
struct TimerForm {
    timer: u32,
}

async fn timer_form(Form(form): Form<TimerForm>) -> impl IntoResponse {
    TIMER_SIGNAL.signal(form.timer);
}

async fn set_timer(sec: u32) -> impl IntoResponse {
    TIMER_SIGNAL.signal(sec);
}
