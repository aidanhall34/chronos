use opentelemetry::global;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::SdkTracerProvider;
use std::{error::Error, thread, time::Duration};
use tracing::{instrument, span, trace, warn};
use tracing_subscriber::prelude::*;

fn init_tracer() -> Result<opentelemetry_sdk::trace::Tracer, Box<dyn Error + Send + Sync>> {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT").unwrap_or_else(|_| "http://localhost:4317".to_string());

    global::set_text_map_propagator(TraceContextPropagator::new());

    let exporter = opentelemetry_otlp::SpanExporter::builder().with_tonic().with_endpoint(endpoint).build()?;

    let provider = SdkTracerProvider::builder().with_batch_exporter(exporter).build();
    global::set_tracer_provider(provider.clone());

    Ok(provider.tracer("opentelemetry_example"))
}

#[instrument]
fn expensive_work() -> &'static str {
    span!(tracing::Level::INFO, "expensive_step_1").in_scope(|| thread::sleep(Duration::from_millis(25)));
    span!(tracing::Level::INFO, "expensive_step_2").in_scope(|| thread::sleep(Duration::from_millis(25)));
    "success"
}

fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    dotenv::dotenv().ok();

    let tracer = init_tracer()?;
    let otel = tracing_opentelemetry::layer().with_tracer(tracer);

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new("info"))
        .with(tracing_subscriber::fmt::layer())
        .with(otel)
        .try_init()?;

    let root = span!(tracing::Level::INFO, "app_start", work_units = 2);
    let _enter = root.enter();

    let work_result = expensive_work();
    warn!("About to exit!");
    trace!("status: {}", work_result);

    Ok(())
}
