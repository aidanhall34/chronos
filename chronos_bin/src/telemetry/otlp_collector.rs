use opentelemetry::trace::TraceError;
use opentelemetry::{
    global,
    sdk::{propagation::TraceContextPropagator, trace as sdktrace},
};
use opentelemetry_otlp::{Protocol, WithExportConfig};

pub struct OtlpCollector {}

impl Default for OtlpCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl OtlpCollector {
    pub fn new() -> Self {
        OtlpCollector {}
    }

    pub fn http_collector_connect(&self, protocol: Protocol) -> Result<sdktrace::Tracer, TraceError> {
        // service name will be picked from  "OTEL_SERVICE_NAME" env variable

        if let Ok(trace_exporter) = std::env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT") {
            global::set_text_map_propagator(TraceContextPropagator::new());
            opentelemetry_otlp::new_pipeline()
                .tracing()
                .with_exporter(opentelemetry_otlp::new_exporter().http().with_protocol(protocol).with_endpoint(trace_exporter))
                .install_batch(opentelemetry::runtime::Tokio)
        } else {
            log::error!("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT not set");

            // trace error
            Err(TraceError::Other(Box::new(std::io::Error::other("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT not set"))))
        }
    }
}
