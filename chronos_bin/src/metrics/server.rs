use crate::metrics::ChronosMetrics;
use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Router};
use prometheus::{Encoder, TextEncoder};
use std::sync::Arc;

async fn metrics_handler(State(metrics): State<Arc<ChronosMetrics>>) -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = metrics.registry.gather();
    let mut buffer = Vec::new();
    match encoder.encode(&metric_families, &mut buffer) {
        Ok(_) => (StatusCode::OK, [("content-type", "text/plain; version=0.0.4; charset=utf-8")], buffer).into_response(),
        Err(e) => {
            log::error!("Failed to encode metrics: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn run_metrics_server(metrics: Arc<ChronosMetrics>, port: u16) {
    let app = Router::new().route("/metrics", get(metrics_handler)).with_state(metrics);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    log::info!("Metrics server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.expect("Failed to bind metrics server port");
    axum::serve(listener, app).await.expect("Metrics server failed");
}
