use chronos_bin::kafka::config::KafkaConfig;
use chronos_bin::kafka::consumer::KafkaConsumer;
use chronos_bin::kafka::producer::KafkaProducer;
use chronos_bin::postgres::config::PgConfig;
use chronos_bin::postgres::pg::Pg;
use chronos_bin::runner::Runner;

use log::{debug, error};
use opentelemetry::global;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::SdkTracerProvider;
use std::error::Error;
use std::sync::Arc;
use tracing_subscriber::prelude::*;

fn init_tracing() -> Result<(), Box<dyn Error + Send + Sync>> {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT").unwrap_or_else(|_| "http://localhost:4317".to_string());
    let service_name = std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "chronos_ex".to_string());

    global::set_text_map_propagator(TraceContextPropagator::new());

    let exporter = opentelemetry_otlp::SpanExporter::builder().with_tonic().with_endpoint(endpoint).build()?;

    let provider = SdkTracerProvider::builder().with_batch_exporter(exporter).build();
    global::set_tracer_provider(provider.clone());

    let tracer = provider.tracer(service_name);

    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(otel_layer)
        .try_init()?;

    Ok(())
}

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenv::dotenv().ok();

    if let Err(e) = init_tracing() {
        error!("failed to initialize telemetry: {}", e);
    }

    let kafka_config = KafkaConfig::from_env();
    let pg_config = PgConfig::from_env();

    let kafka_consumer = KafkaConsumer::new(&kafka_config);
    let kafka_producer = KafkaProducer::new(&kafka_config);
    let data_store = Pg::new(pg_config).await.unwrap();

    let r = Runner {
        data_store: Arc::new(data_store),
        producer: Arc::new(kafka_producer),
        consumer: Arc::new(kafka_consumer),
    };

    debug!("debug logs starting chronos");
    r.run().await;
}
