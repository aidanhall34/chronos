use crate::metrics::ChronosMetrics;
use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Router};
use std::sync::Arc;

async fn metrics_handler(State(metrics): State<Arc<ChronosMetrics>>) -> impl IntoResponse {
    match metrics.render_prometheus() {
        Some(body) => (StatusCode::OK, [("content-type", "text/plain; version=0.0.4; charset=utf-8")], body).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

pub async fn run_metrics_server(metrics: Arc<ChronosMetrics>, host: String, port: u16) {
    if !metrics.is_prometheus() {
        log::info!("Prometheus metrics server disabled because OTEL_METRICS_EXPORTER is not prometheus");
        return;
    }

    let app = Router::new().route("/metrics", get(metrics_handler)).with_state(metrics);

    let addr = format!("{}:{}", host, port);
    log::info!("Metrics server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.expect("Failed to bind metrics server port");
    axum::serve(listener, app).await.expect("Metrics server failed");
}
