use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use super::otlp_collector::OtlpCollector;

pub enum TelemetryCollectorType {
    Otlp,
}

pub struct TelemetryCollector {
    pub collector_type: TelemetryCollectorType,
}

impl Default for TelemetryCollector {
    fn default() -> Self {
        TelemetryCollector {
            collector_type: TelemetryCollectorType::Otlp,
        }
    }
}

impl TelemetryCollector {
    pub fn new(collector_type: TelemetryCollectorType) -> Self {
        TelemetryCollector { collector_type }
    }

    pub fn register_traces(self) {
        let tracer = match self.collector_type {
            TelemetryCollectorType::Otlp => {
                let otlp_collector = OtlpCollector::new();
                let protocol = std::env::var("OTEL_EXPORTER_OTLP_PROTOCOL").unwrap_or_else(|_| "grpc".to_string());
                if protocol.to_lowercase().contains("grpc") {
                    otlp_collector.grpc_collector_connect()
                } else {
                    otlp_collector.http_collector_connect()
                }
            }
        };

        match tracer {
            Ok(tracer) => {
                let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
                let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

                let init_result = tracing_subscriber::registry()
                    .with(filter)
                    .with(tracing_subscriber::fmt::layer())
                    .with(otel_layer)
                    .try_init();

                if let Err(e) = init_result {
                    eprintln!("failed to initialize tracing subscriber: {e}");
                }
            }
            Err(e) => {
                log::error!("error while initializing tracing {}", e);
            }
        }
    }
}
