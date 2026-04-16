use chronos_bin::kafka::config::KafkaConfig;
use chronos_bin::kafka::consumer::KafkaConsumer;
use chronos_bin::kafka::producer::KafkaProducer;
use chronos_bin::metrics::ChronosMetrics;
use chronos_bin::postgres::config::PgConfig;
use chronos_bin::postgres::pg::Pg;
use chronos_bin::runner::Runner;
use log::debug;
use std::sync::Arc;
use tracing_subscriber::prelude::*;

use opentelemetry::trace::TraceError;
use opentelemetry::{
    global,
    sdk::{
        propagation::TraceContextPropagator,
        resource::{
            EnvResourceDetector, OsResourceDetector, ProcessResourceDetector, ResourceDetector, SdkProvidedResourceDetector, TelemetryResourceDetector,
        },
        trace as sdktrace,
    },
};
use opentelemetry_otlp::WithExportConfig;
use std::time::Duration;

use tracing_subscriber::layer::SubscriberExt;

fn init_tracer() -> Result<sdktrace::Tracer, TraceError> {
    let service_name = std::env::var("OTEL_SERVICE_NAME");
    let trace_exporter = std::env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT");
    if service_name.is_err() {
        std::env::set_var("OTEL_SERVICE_NAME", "chronos");
    }
    if trace_exporter.is_ok() {
        global::set_text_map_propagator(TraceContextPropagator::new());
        let os_resource = OsResourceDetector.detect(Duration::from_secs(0));
        let process_resource = ProcessResourceDetector.detect(Duration::from_secs(0));
        let sdk_resource = SdkProvidedResourceDetector.detect(Duration::from_secs(0));
        let env_resource = EnvResourceDetector::new().detect(Duration::from_secs(0));
        let telemetry_resource = TelemetryResourceDetector.detect(Duration::from_secs(0));
        opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(opentelemetry_otlp::new_exporter().http().with_endpoint(format!("{:?}", service_name)))
            .with_trace_config(
                sdktrace::config().with_resource(
                    os_resource
                        .merge(&process_resource)
                        .merge(&sdk_resource)
                        .merge(&env_resource)
                        .merge(&telemetry_resource),
                ),
            )
            .install_batch(opentelemetry::runtime::Tokio)
    } else {
        log::error!("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT not set");

        // trace error
        Err(TraceError::Other(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT not set",
        ))))
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenv::dotenv().ok();

    let tracer = init_tracer().expect("failed to install tracing");

    //creating a layer for Otel
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // let subscriber = Registry::default().with(otel_layer);

    //subscribing to tracing with opentelemetry
    match tracing_subscriber::registry().with(otel_layer).try_init() {
        Ok(_) => {}
        Err(e) => {
            println!("error while initializing tracing {}", e);
        }
    }
    // Initialise telemetry pipeline
    // init_pipeline();
    // let tracer = tracer("chronos_tracer");

    let kafka_config = KafkaConfig::from_env();
    let pg_config = PgConfig::from_env();

    let kafka_consumer = KafkaConsumer::new(&kafka_config);
    let kafka_producer = KafkaProducer::new(&kafka_config);
    let data_store = Pg::new(pg_config).await.unwrap();

    let metrics = Arc::new(ChronosMetrics::new().expect("Failed to initialize metrics registry"));

    let r = Runner {
        data_store: Arc::new(data_store),
        producer: Arc::new(kafka_producer),
        consumer: Arc::new(kafka_consumer),
        metrics,
    };

    debug!("debug logs starting chronos");

    r.run().await;
}
