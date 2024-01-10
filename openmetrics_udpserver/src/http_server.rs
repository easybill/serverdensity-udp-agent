use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::get;
use axum::{debug_handler, Router};
use prometheus_client::encoding::text::encode;
use prometheus_client::registry::Registry;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::config::Config;
use crate::METRIC_COUNTER_REQUESTS;

#[derive(Clone)]
struct HttpServerState {
    metric_registry: Arc<RwLock<Registry>>,
}

#[debug_handler]
async fn get_metrics(
    State(state): State<Arc<HttpServerState>>,
) -> Result<Response<String>, StatusCode> {
    METRIC_COUNTER_REQUESTS.inc();

    let registry = state.metric_registry.read().await;
    let body = {
        let mut buffer = String::new();
        if encode(&mut buffer, &registry).is_err() {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
        buffer
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(
            "Content-Type",
            "application/openmetrics-text; version=1.0.0; charset=utf-8",
        )
        .body(body)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) fn bind(
    config: &Config,
    metric_registry: Arc<RwLock<Registry>>,
) -> JoinHandle<Result<(), std::io::Error>> {
    let state = Arc::new(HttpServerState { metric_registry });
    let router = Router::new()
        .route("/metrics", get(get_metrics))
        .with_state(state);

    let bind_addr = config
        .http_bind
        .parse::<SocketAddr>()
        .expect("Unable to parse http bind address");
    tokio::spawn(async move {
        let listener = TcpListener::bind(bind_addr)
            .await
            .expect("Unable to bind TCP listener");
        axum::serve(listener, router).await
    })
}
