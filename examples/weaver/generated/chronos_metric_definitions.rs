// Generated from examples/weaver/registry/chronos/metrics.yaml by OpenTelemetry Weaver.
// Do not edit by hand.

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MetricId {
    MsgConsumeLatency,
    MsgConsumed,
    MsgJitter,
    MsgProcessLatency,
    MsgReset,
    MsgWaitTime,
}

#[derive(Clone, Copy, Debug)]
pub enum MetricKind {
    Counter,
    Histogram,
}

#[derive(Clone, Copy, Debug)]
pub struct MetricDefinition {
    pub id: MetricId,
    pub otel_name: &'static str,
    pub prometheus_name: &'static str,
    pub description: &'static str,
    pub unit: Option<&'static str>,
    pub label_names: &'static [&'static str],
    pub kind: MetricKind,
    pub buckets: Option<&'static [f64]>,
}

pub const METRIC_DEFINITIONS: &[MetricDefinition] = &[
    MetricDefinition {
        id: MetricId::MsgConsumeLatency,
        otel_name: "chronos.message.consume.duration",
        prometheus_name: "chronos_message_consume_duration_seconds",
        description: "Duration of handle_message() in message_receiver.",
        unit: Some("s"),
        label_names: &["chronos.message.destination", "chronos.operation.status"],
        kind: MetricKind::Histogram,
        buckets: Some(&[0.001, 0.002, 0.004, 0.008, 0.016, 0.032, 0.064, 0.128, 0.256, 0.512, 1.024, 2.048]),
    },
    MetricDefinition {
        id: MetricId::MsgConsumed,
        otel_name: "chronos.message.consumed",
        prometheus_name: "chronos_messages_consumed_total",
        description: "Total number of Chronos input messages consumed.",
        unit: Some("{message}"),
        label_names: &["chronos.message.destination", "chronos.operation.status"],
        kind: MetricKind::Counter,
        buckets: None,
    },
    MetricDefinition {
        id: MetricId::MsgJitter,
        otel_name: "chronos.message.jitter",
        prometheus_name: "chronos_message_jitter_seconds",
        description: "Difference between actual publish time and client-requested deadline.",
        unit: Some("s"),
        label_names: &[],
        kind: MetricKind::Histogram,
        buckets: Some(&[0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]),
    },
    MetricDefinition {
        id: MetricId::MsgProcessLatency,
        otel_name: "chronos.message.process.duration",
        prometheus_name: "chronos_message_process_duration_seconds",
        description: "Duration of processor_message_ready() loop in message_processor.",
        unit: Some("s"),
        label_names: &["chronos.operation.status", "chronos.processor.returned"],
        kind: MetricKind::Histogram,
        buckets: Some(&[0.001, 0.002, 0.004, 0.008, 0.016, 0.032, 0.064, 0.128, 0.256, 0.512, 1.024, 2.048]),
    },
    MetricDefinition {
        id: MetricId::MsgReset,
        otel_name: "chronos.message.reset",
        prometheus_name: "chronos_messages_reset_total",
        description: "Number of records reset by reset_to_init_db() in the monitor task.",
        unit: Some("{message}"),
        label_names: &[],
        kind: MetricKind::Counter,
        buckets: None,
    },
    MetricDefinition {
        id: MetricId::MsgWaitTime,
        otel_name: "chronos.message.wait.duration",
        prometheus_name: "chronos_message_wait_duration_seconds",
        description: "Time a message spent in the Kafka input queue before processing.",
        unit: Some("s"),
        label_names: &[],
        kind: MetricKind::Histogram,
        buckets: Some(&[0.1, 0.2, 0.4, 0.8, 1.6, 3.2, 6.4, 12.8, 25.6, 51.2, 102.4, 204.8, 409.6, 819.2]),
    },
];
