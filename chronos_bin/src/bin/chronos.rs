use chronos_bin::kafka::config::KafkaConfig;
use chronos_bin::kafka::consumer::KafkaConsumer;
use chronos_bin::kafka::producer::KafkaProducer;
use chronos_bin::metrics::ChronosMetrics;
use chronos_bin::postgres::config::PgConfig;
use chronos_bin::postgres::pg::Pg;
use chronos_bin::runner::Runner;
use chronos_bin::telemetry::register_telemetry::{TelemetryCollector, TelemetryCollectorType};
use log::{debug, info};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenvy::dotenv().ok();

    let protocol = std::env::var("OTEL_EXPORTER_OTLP_PROTOCOL").unwrap_or_else(|_| "http/json".to_string());

    let tracing_opentelemetry = TelemetryCollector::new(protocol, TelemetryCollectorType::Otlp);
    tracing_opentelemetry.register_traces();

    let kafka_config = KafkaConfig::from_env();
    let pg_config = PgConfig::from_env();

    let kafka_consumer = KafkaConsumer::new(&kafka_config);
    let kafka_producer = KafkaProducer::new(&kafka_config);
    let data_store = match Pg::new(pg_config).await {
        Ok(pg) => pg,
        Err(e) => loop {
            log::error!("couldnt connect to PG DB due to error::{} will retry ", e);
            tokio::time::sleep(Duration::from_secs(10)).await;
            let pg_config = PgConfig::from_env();
            match Pg::new(pg_config).await {
                Ok(pg) => pg,
                Err(e) => {
                    log::error!("error while creating PG intance {}", e);
                    continue;
                }
            };
        },
    };

    let metrics = Arc::new(ChronosMetrics::new().expect("Failed to initialize metrics registry"));

    info!("starting chronos establish connections");
    let r = Runner {
        data_store: Arc::new(data_store),
        producer: Arc::new(kafka_producer),
        consumer: Arc::new(kafka_consumer),
        metrics,
    };

    debug!("debug logs starting chronos");

    r.run().await;
}
