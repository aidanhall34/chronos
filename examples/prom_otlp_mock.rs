//! Mock design for a Chronos metrics abstraction that can export through either
//! the Prometheus client library or OpenTelemetry OTLP metrics.
//!
//! Selection is intentionally driven by the standard OpenTelemetry metric
//! exporter variable:
//!
//! - `OTEL_METRICS_EXPORTER=prometheus` uses the `prometheus` crate registry.
//! - `OTEL_METRICS_EXPORTER=otlp` uses the OTLP gRPC exporter.
//! - unset defaults to Prometheus for local compatibility.
//!
//! This file is a design sketch for the Chronos rewrite, not wired into the
//! runtime yet. The important shape is that metric definitions live once in
//! `MetricDefinition`, while the backend-specific registrations stay behind the
//! `MetricsBackend` interface.

use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::Duration;

use opentelemetry::global;
use opentelemetry::metrics::{Counter as OtlpCounter, Histogram as OtlpHistogram, Unit};
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use prometheus::{histogram_opts, opts, CounterVec as PromCounterVec, HistogramVec as PromHistogramVec, Registry};

const OTEL_METRICS_EXPORTER: &str = "OTEL_METRICS_EXPORTER";
const OTEL_EXPORTER_OTLP_ENDPOINT: &str = "OTEL_EXPORTER_OTLP_ENDPOINT";
const OTEL_EXPORTER_OTLP_METRICS_ENDPOINT: &str = "OTEL_EXPORTER_OTLP_METRICS_ENDPOINT";
const OTEL_EXPORTER_OTLP_PROTOCOL: &str = "OTEL_EXPORTER_OTLP_PROTOCOL";
const OTEL_EXPORTER_OTLP_METRICS_PROTOCOL: &str = "OTEL_EXPORTER_OTLP_METRICS_PROTOCOL";
const OTEL_METRIC_EXPORT_INTERVAL: &str = "OTEL_METRIC_EXPORT_INTERVAL";
const OTEL_EXPORTER_PROMETHEUS_HOST: &str = "OTEL_EXPORTER_PROMETHEUS_HOST";
const OTEL_EXPORTER_PROMETHEUS_PORT: &str = "OTEL_EXPORTER_PROMETHEUS_PORT";

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum MetricId {
    MsgConsumed,
    MsgConsumeLatency,
}

#[derive(Clone, Copy, Debug)]
enum MetricKind {
    Counter,
    Histogram,
}

#[derive(Clone, Copy, Debug)]
struct MetricDefinition {
    id: MetricId,
    otel_name: &'static str,
    prometheus_name: &'static str,
    description: &'static str,
    unit: Option<&'static str>,
    attribute_names: &'static [&'static str],
    prometheus_label_names: &'static [&'static str],
    kind: MetricKind,
}

const METRIC_DEFINITIONS: &[MetricDefinition] = &[
    MetricDefinition {
        id: MetricId::MsgConsumed,
        otel_name: "messaging.client.consumed.messages",
        prometheus_name: "messaging_client_consumed_messages",
        description: "Total number of Chronos input messages consumed",
        unit: Some("{message}"),
        attribute_names: &["messaging.system", "messaging.operation.name", "messaging.destination.name"],
        prometheus_label_names: &["messaging_system", "messaging_operation_name", "messaging_destination_name"],
        kind: MetricKind::Counter,
    },
    MetricDefinition {
        id: MetricId::MsgConsumeLatency,
        otel_name: "messaging.process.duration",
        prometheus_name: "messaging_process_duration_seconds",
        description: "Time spent handling a consumed Chronos message",
        unit: Some("s"),
        attribute_names: &["messaging.system", "messaging.operation.name", "messaging.destination.name"],
        prometheus_label_names: &["messaging_system", "messaging_operation_name", "messaging_destination_name"],
        kind: MetricKind::Histogram,
    },
];

trait MetricsBackend: Send + Sync {
    fn inc_counter(&self, id: MetricId, value: u64, labels: &[(&'static str, String)]);
    fn observe_histogram(&self, id: MetricId, value: f64, labels: &[(&'static str, String)]);
    fn render_prometheus(&self) -> Option<String>;
    fn shutdown(&self);
}

struct ChronosMetrics {
    backend: Box<dyn MetricsBackend>,
}

impl ChronosMetrics {
    fn from_env() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let backend: Box<dyn MetricsBackend> = match MetricsExporter::from_env()? {
            MetricsExporter::Prometheus => Box::new(PrometheusMetricsBackend::new()?),
            MetricsExporter::Otlp => Box::new(OtlpMetricsBackend::new()?),
        };

        Ok(Self { backend })
    }

    fn message_consumed(&self, destination: &'static str) {
        self.backend.inc_counter(
            MetricId::MsgConsumed,
            1,
            &[
                ("messaging.system", "kafka".to_string()),
                ("messaging.operation.name", "receive".to_string()),
                ("messaging.destination.name", destination.to_string()),
            ],
        );
    }

    fn consume_latency(&self, seconds: f64, destination: &'static str) {
        self.backend.observe_histogram(
            MetricId::MsgConsumeLatency,
            seconds,
            &[
                ("messaging.system", "kafka".to_string()),
                ("messaging.operation.name", "process".to_string()),
                ("messaging.destination.name", destination.to_string()),
            ],
        );
    }

    fn record_cycle(&self, cycle: u64) {
        let destination = if cycle.is_multiple_of(2) { "chronos-input" } else { "chronos-retry" };
        let latency_seconds = 0.005 + ((cycle % 20) as f64 * 0.0025);

        self.message_consumed(destination);
        self.consume_latency(latency_seconds, destination);
    }

    fn prometheus_text(&self) -> Option<String> {
        self.backend.render_prometheus()
    }

    fn shutdown(&self) {
        self.backend.shutdown();
    }
}

struct MockRuntimeConfig {
    interval: Duration,
    prometheus_host: String,
    prometheus_port: u16,
}

impl MockRuntimeConfig {
    fn from_env() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self {
            interval: env_duration_ms(OTEL_METRIC_EXPORT_INTERVAL, 1_000)?,
            prometheus_host: env::var(OTEL_EXPORTER_PROMETHEUS_HOST).unwrap_or_else(|_| "127.0.0.1".to_string()),
            prometheus_port: env::var(OTEL_EXPORTER_PROMETHEUS_PORT)
                .unwrap_or_else(|_| "9092".to_string())
                .parse()
                .map_err(|err| format!("invalid {OTEL_EXPORTER_PROMETHEUS_PORT}: {err}"))?,
        })
    }
}

fn env_duration_ms(name: &'static str, default_ms: u64) -> Result<Duration, Box<dyn std::error::Error + Send + Sync>> {
    let millis = env::var(name)
        .unwrap_or_else(|_| default_ms.to_string())
        .parse()
        .map_err(|err| format!("invalid {name}: {err}"))?;
    Ok(Duration::from_millis(millis))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MetricsExporter {
    Prometheus,
    Otlp,
}

impl MetricsExporter {
    fn from_env() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        match env::var(OTEL_METRICS_EXPORTER).unwrap_or_else(|_| "prometheus".to_string()).as_str() {
            "prometheus" => Ok(Self::Prometheus),
            "otlp" => {
                require_grpc_protocol()?;
                Ok(Self::Otlp)
            }
            "none" => Err("metrics exporter disabled by OTEL_METRICS_EXPORTER=none".into()),
            other => Err(format!("unsupported {OTEL_METRICS_EXPORTER} value: {other}").into()),
        }
    }
}

fn require_grpc_protocol() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let protocol = env::var(OTEL_EXPORTER_OTLP_METRICS_PROTOCOL)
        .or_else(|_| env::var(OTEL_EXPORTER_OTLP_PROTOCOL))
        .unwrap_or_else(|_| "grpc".to_string());

    if protocol == "grpc" {
        Ok(())
    } else {
        Err(format!("unsupported OTLP metrics protocol {protocol:?}; use grpc for this design").into())
    }
}

struct PrometheusMetricsBackend {
    registry: Registry,
    counters: HashMap<MetricId, PromCounterVec>,
    histograms: HashMap<MetricId, PromHistogramVec>,
}

impl PrometheusMetricsBackend {
    fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();
        let mut counters = HashMap::new();
        let mut histograms = HashMap::new();

        for definition in METRIC_DEFINITIONS {
            match definition.kind {
                MetricKind::Counter => {
                    let metric = PromCounterVec::new(opts!(definition.prometheus_name, definition.description), definition.prometheus_label_names)?;
                    registry.register(Box::new(metric.clone()))?;
                    counters.insert(definition.id, metric);
                }
                MetricKind::Histogram => {
                    let metric = PromHistogramVec::new(
                        histogram_opts!(definition.prometheus_name, definition.description),
                        definition.prometheus_label_names,
                    )?;
                    registry.register(Box::new(metric.clone()))?;
                    histograms.insert(definition.id, metric);
                }
            }
        }

        Ok(Self {
            registry,
            counters,
            histograms,
        })
    }
}

impl MetricsBackend for PrometheusMetricsBackend {
    fn inc_counter(&self, id: MetricId, value: u64, labels: &[(&'static str, String)]) {
        if let Some(counter) = self.counters.get(&id) {
            let label_values = prometheus_label_values(id, labels);
            if let Ok(metric) = counter.get_metric_with_label_values(&label_values) {
                metric.inc_by(value as f64);
            }
        }
    }

    fn observe_histogram(&self, id: MetricId, value: f64, labels: &[(&'static str, String)]) {
        if let Some(histogram) = self.histograms.get(&id) {
            let label_values = prometheus_label_values(id, labels);
            if let Ok(metric) = histogram.get_metric_with_label_values(&label_values) {
                metric.observe(value);
            }
        }
    }

    fn render_prometheus(&self) -> Option<String> {
        use prometheus::{Encoder, TextEncoder};

        let encoder = TextEncoder::new();
        let mut buffer = Vec::new();
        encoder.encode(&self.registry.gather(), &mut buffer).ok()?;
        String::from_utf8(buffer).ok()
    }

    fn shutdown(&self) {}
}

struct OtlpMetricsBackend {
    provider: opentelemetry_sdk::metrics::MeterProvider,
    counters: HashMap<MetricId, OtlpCounter<u64>>,
    histograms: HashMap<MetricId, OtlpHistogram<f64>>,
}

impl OtlpMetricsBackend {
    fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let endpoint = env::var(OTEL_EXPORTER_OTLP_METRICS_ENDPOINT)
            .or_else(|_| env::var(OTEL_EXPORTER_OTLP_ENDPOINT))
            .unwrap_or_else(|_| "http://127.0.0.1:4317".to_string());
        let exporter = opentelemetry_otlp::new_exporter().tonic().with_env().with_endpoint(endpoint);
        let provider = opentelemetry_otlp::new_pipeline()
            .metrics(opentelemetry::runtime::Tokio)
            .with_exporter(exporter)
            .build()?;

        global::set_meter_provider(provider.clone());
        let meter = global::meter("chronos");

        let mut counters = HashMap::new();
        let mut histograms = HashMap::new();

        for definition in METRIC_DEFINITIONS {
            match definition.kind {
                MetricKind::Counter => {
                    let mut builder = meter.u64_counter(definition.otel_name).with_description(definition.description);
                    if let Some(unit) = definition.unit {
                        builder = builder.with_unit(Unit::new(unit));
                    }
                    counters.insert(definition.id, builder.init());
                }
                MetricKind::Histogram => {
                    let mut builder = meter.f64_histogram(definition.otel_name).with_description(definition.description);
                    if let Some(unit) = definition.unit {
                        builder = builder.with_unit(Unit::new(unit));
                    }
                    histograms.insert(definition.id, builder.init());
                }
            }
        }

        Ok(Self {
            provider,
            counters,
            histograms,
        })
    }
}

impl MetricsBackend for OtlpMetricsBackend {
    fn inc_counter(&self, id: MetricId, value: u64, labels: &[(&'static str, String)]) {
        if let Some(counter) = self.counters.get(&id) {
            counter.add(value, &labels_to_key_values(labels));
        }
    }

    fn observe_histogram(&self, id: MetricId, value: f64, labels: &[(&'static str, String)]) {
        if let Some(histogram) = self.histograms.get(&id) {
            histogram.record(value, &labels_to_key_values(labels));
        }
    }

    fn render_prometheus(&self) -> Option<String> {
        None
    }

    fn shutdown(&self) {
        if let Err(err) = self.provider.force_flush(&opentelemetry::Context::current()) {
            eprintln!("failed to flush OTLP metrics: {err}");
        }
        if let Err(err) = self.provider.shutdown() {
            eprintln!("failed to shut down OTLP metrics provider: {err}");
        }
    }
}

fn labels_to_key_values(labels: &[(&'static str, String)]) -> Vec<KeyValue> {
    labels.iter().map(|(key, value)| KeyValue::new(*key, value.clone())).collect()
}

fn prometheus_label_values<'a>(id: MetricId, labels: &'a [(&'static str, String)]) -> Vec<&'a str> {
    let Some(definition) = METRIC_DEFINITIONS.iter().find(|definition| definition.id == id) else {
        return Vec::new();
    };

    definition
        .attribute_names
        .iter()
        .map(|name| {
            labels
                .iter()
                .find(|(label_name, _)| label_name == name)
                .map(|(_, value)| value.as_str())
                .unwrap_or("unknown")
        })
        .collect()
}

async fn spawn_prometheus_server(
    metrics: Arc<ChronosMetrics>,
    host: String,
    port: u16,
) -> Result<tokio::task::JoinHandle<()>, Box<dyn std::error::Error + Send + Sync>> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}")).await?;
    eprintln!("Prometheus metrics mock listening on http://{host}:{port}/metrics");

    Ok(tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                continue;
            };
            let metrics = Arc::clone(&metrics);
            tokio::spawn(async move {
                let mut request = [0_u8; 1024];
                let bytes_read = stream.read(&mut request).await.unwrap_or(0);
                let request_line = String::from_utf8_lossy(&request[..bytes_read]);
                let (status, body) = if request_line.starts_with("GET /metrics ") {
                    ("200 OK", metrics.prometheus_text().unwrap_or_default())
                } else {
                    ("404 Not Found", "not found\n".to_string())
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\ncontent-type: text/plain; version=0.0.4; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes()).await;
            });
        }
    }))
}

async fn run_workload(metrics: Arc<ChronosMetrics>, config: &MockRuntimeConfig) {
    let mut cycle = 0_u64;
    loop {
        cycle += 1;
        metrics.record_cycle(cycle);

        tokio::time::sleep(config.interval).await;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let exporter = MetricsExporter::from_env()?;
    let config = MockRuntimeConfig::from_env()?;
    let metrics = Arc::new(ChronosMetrics::from_env()?);

    let prometheus_server = if exporter == MetricsExporter::Prometheus {
        let metrics_for_server = Arc::clone(&metrics);
        Some(spawn_prometheus_server(metrics_for_server, config.prometheus_host.clone(), config.prometheus_port).await?)
    } else {
        None
    };

    eprintln!("Metrics mock running until interrupted");

    tokio::select! {
        _ = run_workload(Arc::clone(&metrics), &config) => {}
        result = tokio::signal::ctrl_c() => {
            result?;
        }
    }

    if exporter == MetricsExporter::Otlp {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }

    metrics.shutdown();
    if let Some(server) = prometheus_server {
        server.abort();
    }
    Ok(())
}
