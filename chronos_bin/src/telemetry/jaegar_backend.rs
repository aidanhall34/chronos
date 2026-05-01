use opentelemetry_api::trace::TraceError;
use opentelemetry_sdk::trace::Tracer;

pub fn instrument_jaegar_pipleline() -> Result<Tracer, TraceError> {
    let service_name = std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| {
        let service_name = "chronos".to_string();
        std::env::set_var("OTEL_SERVICE_NAME", &service_name);
        service_name
    });
    opentelemetry_jaeger::new_agent_pipeline().with_service_name(service_name).install_simple()
}
