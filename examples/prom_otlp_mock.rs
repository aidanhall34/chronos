//! Mock runner for exercising Chronos metrics through Prometheus or OTLP.
//!
//! This intentionally uses the production `ChronosMetrics` facade so Weaver
//! live checks validate the same generated metric definitions as the runtime.

use std::env;
use std::sync::Arc;
use std::time::Duration;

use chronos_bin::metrics::ChronosMetrics;

const OTEL_METRICS_EXPORTER: &str = "OTEL_METRICS_EXPORTER";
const OTEL_METRIC_EXPORT_INTERVAL: &str = "OTEL_METRIC_EXPORT_INTERVAL";
const OTEL_EXPORTER_PROMETHEUS_HOST: &str = "OTEL_EXPORTER_PROMETHEUS_HOST";
const OTEL_EXPORTER_PROMETHEUS_PORT: &str = "OTEL_EXPORTER_PROMETHEUS_PORT";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MetricsExporter {
    Prometheus,
    Otlp,
}

impl MetricsExporter {
    fn from_env() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        match env::var(OTEL_METRICS_EXPORTER).unwrap_or_else(|_| "prometheus".to_string()).as_str() {
            "prometheus" => Ok(Self::Prometheus),
            "otlp" => Ok(Self::Otlp),
            "none" => Err("metrics exporter disabled by OTEL_METRICS_EXPORTER=none".into()),
            other => Err(format!("unsupported {OTEL_METRICS_EXPORTER} value: {other}").into()),
        }
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
                    ("200 OK", metrics.render_prometheus().unwrap_or_default())
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

        let consume_destination = if cycle.is_multiple_of(2) { "kafka" } else { "postgres" };
        let consume_status = if cycle.is_multiple_of(5) { "fail" } else { "pass" };
        let process_returned = cycle.is_multiple_of(3);
        let process_status = if cycle.is_multiple_of(7) { "fail" } else { "pass" };
        let duration_seconds = 0.005 + ((cycle % 20) as f64 * 0.0025);

        metrics.observe_consume_latency(duration_seconds, consume_destination, consume_status);
        metrics.observe_process_latency(duration_seconds * 1.5, process_returned, process_status);
        metrics.observe_wait_time(0.1 + ((cycle % 10) as f64 * 0.05));
        metrics.observe_jitter(0.01 + ((cycle % 10) as f64 * 0.025));
        metrics.messages_reset(1);

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
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    metrics.shutdown();
    if let Some(server) = prometheus_server {
        server.abort();
    }
    Ok(())
}
