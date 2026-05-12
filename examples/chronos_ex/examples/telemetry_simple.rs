use opentelemetry::global;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tokio::time::Duration;
use tracing::{info, info_span, instrument};
use tracing_subscriber::prelude::*;

fn init_tracer() -> Result<opentelemetry_sdk::trace::Tracer, Box<dyn std::error::Error + Send + Sync>> {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT").unwrap_or_else(|_| "http://localhost:4317".to_string());

    global::set_text_map_propagator(TraceContextPropagator::new());

    let exporter = opentelemetry_otlp::SpanExporter::builder().with_tonic().with_endpoint(endpoint).build()?;

    let provider = SdkTracerProvider::builder().with_batch_exporter(exporter).build();
    global::set_tracer_provider(provider.clone());

    Ok(provider.tracer("telemetry_simple"))
}

#[instrument]
async fn worker() {
    let span = info_span!("worker_span");
    let _guard = span.enter();
    info!("worker started");
    tokio::time::sleep(Duration::from_millis(200)).await;
    info!("worker finished");
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let tracer = init_tracer().expect("failed to init tracer");
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new("info"))
        .with(tracing_subscriber::fmt::layer())
        .with(otel_layer)
        .try_init()
        .ok();

    worker().await;
    tokio::time::sleep(Duration::from_secs(1)).await;
}
