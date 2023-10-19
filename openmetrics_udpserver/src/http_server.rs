use crate::config::Config;
use actix_web::web::Data;
use actix_web::{get, App, HttpResponse, HttpServer};
use prometheus_client::encoding::text::encode;
use prometheus_client::registry::Registry;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
struct HttpServerState {
    metric_registry: Arc<RwLock<Registry>>,
}

#[get("/metrics")]
async fn get_metrics(state: Data<HttpServerState>) -> HttpResponse {
    if let Ok(registry) = state.metric_registry.try_read() {
        let mut buffer = String::new();
        if encode(&mut buffer, &registry).is_ok() {
            return HttpResponse::Ok()
                .content_type("application/openmetrics-text; version=1.0.0; charset=utf-8")
                .body(buffer);
        }
    }

    HttpResponse::InternalServerError().body(String::from("Unable to access metric registry"))
}

pub(crate) async fn bind(
    config: Config,
    metric_registry: Arc<RwLock<Registry>>,
) -> std::io::Result<()> {
    let state = HttpServerState { metric_registry };

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(state.clone()))
            .service(get_metrics)
    })
    .bind(config.http_bind)?
    .run()
    .await
}
