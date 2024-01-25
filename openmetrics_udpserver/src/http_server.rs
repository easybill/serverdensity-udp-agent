use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::State;
use axum::http::header::ACCEPT;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, Response};
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

async fn get_index() -> Html<String> {
    Html("<html><body><a href=\"/metrics\">/metrics</a></body></html>".to_string())
}

#[debug_handler]
async fn get_metrics(
    headers: HeaderMap,
    State(state): State<Arc<HttpServerState>>,
) -> Result<Response<String>, StatusCode> {
    METRIC_COUNTER_REQUESTS.inc();

    // very basic accept header matching to return text/plain in case no open metrics format was requested
    let accept = headers
        .get(ACCEPT)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("text/plain");
    let response_content_type = if accept.contains("application/openmetrics-text") {
        "application/openmetrics-text; version=1.0.0"
    } else {
        "text/plain"
    };

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
            format!("{}; charset=utf-8", response_content_type),
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
        .route("/", get(get_index))
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
