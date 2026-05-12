use opentelemetry::global;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::{SdkTracerProvider, Tracer};

pub struct OtlpCollector;

impl OtlpCollector {
    pub fn new() -> Self {
        Self
    }

    pub fn grpc_collector_connect(&self) -> Result<Tracer, Box<dyn std::error::Error + Send + Sync>> {
        let endpoint = std::env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT")
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT not set"))?;

        global::set_text_map_propagator(TraceContextPropagator::new());

        let exporter = opentelemetry_otlp::SpanExporter::builder().with_tonic().with_endpoint(endpoint).build()?;

        let provider = SdkTracerProvider::builder().with_batch_exporter(exporter).build();

        global::set_tracer_provider(provider.clone());

        let service_name = std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "chronos".to_string());
        Ok(provider.tracer(service_name))
    }

    pub fn http_collector_connect(&self) -> Result<Tracer, Box<dyn std::error::Error + Send + Sync>> {
        let endpoint = std::env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT")
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT not set"))?;

        global::set_text_map_propagator(TraceContextPropagator::new());

        let exporter = opentelemetry_otlp::SpanExporter::builder().with_http().with_endpoint(endpoint).build()?;

        let provider = SdkTracerProvider::builder().with_batch_exporter(exporter).build();

        global::set_tracer_provider(provider.clone());

        let service_name = std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "chronos".to_string());
        Ok(provider.tracer(service_name))
    }
}
