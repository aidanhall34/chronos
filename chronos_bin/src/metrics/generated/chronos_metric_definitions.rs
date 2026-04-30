// Generated from chronos_bin/src/metrics/spec.yaml.
// Do not edit by hand.
//
// This generated definition table is intentionally not imported by
// chronos_bin/src/metrics/mod.rs yet. The current hand-written Prometheus
// registry remains the runtime implementation until the generated registry
// replacement is wired in explicitly.

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MetricId {
    MsgConsumeLatency,
    MsgJitter,
    MsgProcessLatency,
    MsgReset,
    MsgWaitTime,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetricKind {
    Counter,
    Histogram,
}

#[derive(Clone, Copy, Debug)]
pub struct MetricDefinition {
    pub id: MetricId,
    pub rust_name: &'static str,
    pub prometheus_name: &'static str,
    pub otel_name: &'static str,
    pub description: &'static str,
    pub unit: Option<&'static str>,
    pub label_names: &'static [&'static str],
    pub otel_label_names: &'static [&'static str],
    pub kind: MetricKind,
    pub buckets: Option<&'static [f64]>,
    pub prewarm_label_values: &'static [&'static [&'static str]],
}

pub const METRIC_DEFINITIONS: &[MetricDefinition] = &[
    MetricDefinition {
        id: MetricId::MsgConsumeLatency,
        rust_name: "msg_consume_latency",
        prometheus_name: "msg_consume_latency",
        otel_name: "chronos.message.consume.duration",
        description: "Duration of message_receiver::MessageReceiver::handle_message().",
        unit: Some("s"),
        label_names: &["destination", "status"],
        otel_label_names: &["chronos.destination", "chronos.status"],
        kind: MetricKind::Histogram,
        buckets: Some(&[0.001, 0.002, 0.004, 0.008, 0.016, 0.032, 0.064, 0.128, 0.256, 0.512, 1.024, 2.048]),
        prewarm_label_values: &[&["kafka", "pass"], &["kafka", "fail"], &["postgres", "pass"], &["postgres", "fail"]],
    },
    MetricDefinition {
        id: MetricId::MsgJitter,
        rust_name: "msg_jitter",
        prometheus_name: "msg_jitter",
        otel_name: "chronos.message.jitter",
        description: "Difference between actual publish time and client-requested deadline.",
        unit: Some("s"),
        label_names: &[],
        otel_label_names: &[],
        kind: MetricKind::Histogram,
        buckets: Some(&[0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]),
        prewarm_label_values: &[],
    },
    MetricDefinition {
        id: MetricId::MsgProcessLatency,
        rust_name: "msg_process_latency",
        prometheus_name: "msg_process_latency",
        otel_name: "chronos.message.process.duration",
        description: "Duration of message_processor::MessageProcessor::processor_message_ready().",
        unit: Some("s"),
        label_names: &["returned", "status"],
        otel_label_names: &["chronos.processor.returned", "chronos.status"],
        kind: MetricKind::Histogram,
        buckets: Some(&[0.001, 0.002, 0.004, 0.008, 0.016, 0.032, 0.064, 0.128, 0.256, 0.512, 1.024, 2.048]),
        prewarm_label_values: &[&["true", "pass"], &["true", "fail"], &["false", "pass"], &["false", "fail"]],
    },
    MetricDefinition {
        id: MetricId::MsgReset,
        rust_name: "msg_reset",
        prometheus_name: "msg_reset",
        otel_name: "chronos.message.reset",
        description: "Number of records reset by postgres::pg::Pg::reset_to_init_db() in the monitor task.",
        unit: Some("{message}"),
        label_names: &[],
        otel_label_names: &[],
        kind: MetricKind::Counter,
        buckets: None,
        prewarm_label_values: &[],
    },
    MetricDefinition {
        id: MetricId::MsgWaitTime,
        rust_name: "msg_wait_time",
        prometheus_name: "msg_wait_time",
        otel_name: "chronos.message.wait.duration",
        description: "Time a message spent in the Kafka input queue before processing.",
        unit: Some("s"),
        label_names: &[],
        otel_label_names: &[],
        kind: MetricKind::Histogram,
        buckets: Some(&[0.1, 0.2, 0.4, 0.8, 1.6, 3.2, 6.4, 12.8, 25.6, 51.2, 102.4, 204.8, 409.6, 819.2]),
        prewarm_label_values: &[],
    },
];
