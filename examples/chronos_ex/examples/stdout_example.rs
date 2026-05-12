use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_stdout as stdout;
use tracing::{error, span};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;

fn main() {
    let provider = SdkTracerProvider::builder().with_simple_exporter(stdout::SpanExporter::default()).build();

    let tracer = provider.tracer("stdout_example");
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    let subscriber = Registry::default().with(telemetry);

    tracing::subscriber::with_default(subscriber, || {
        let root = span!(tracing::Level::TRACE, "app_start", work_units = 2);
        let _enter = root.enter();
        error!("This event will be logged in the root span.");
    });
}
