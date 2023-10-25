use crate::config::Config;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::get;
use axum::{debug_handler, Router, Server};
use prometheus_client::encoding::text::encode;
use prometheus_client::registry::Registry;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

#[derive(Clone)]
struct HttpServerState {
    metric_registry: Arc<RwLock<Registry>>,
}

#[debug_handler]
async fn get_metrics(
    State(state): State<Arc<HttpServerState>>,
) -> Result<Response<String>, StatusCode> {
    if let Ok(registry) = state.metric_registry.try_read() {
        let mut buffer = String::new();
        if encode(&mut buffer, &registry).is_ok() {
            return Response::builder()
                .status(StatusCode::OK)
                .header(
                    "Content-Type",
                    "application/openmetrics-text; version=1.0.0; charset=utf-8",
                )
                .body(buffer)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    Response::builder()
        .status(StatusCode::LOCKED)
        .body(String::from("Unable to access metric registry"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) fn bind(
    config: Config,
    metric_registry: Arc<RwLock<Registry>>,
) -> JoinHandle<Result<(), hyper::Error>> {
    let state = Arc::new(HttpServerState { metric_registry });
    let router = Router::new()
        .route("/metrics", get(get_metrics))
        .with_state(state);

    let bind_addr = config
        .http_bind
        .parse::<SocketAddr>()
        .expect("Unable to parse http bind address");
    tokio::spawn(Server::bind(&bind_addr).serve(router.into_make_service()))
}
