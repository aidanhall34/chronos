// Generated from examples/weaver/registry/chronos/metrics.yaml by OpenTelemetry Weaver.
// Do not edit by hand.

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MetricId {
    MsgConsumeLatency,
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

impl MetricKind {
    pub fn is_counter(self) -> bool {
        matches!(self, Self::Counter)
    }

    pub fn is_histogram(self) -> bool {
        matches!(self, Self::Histogram)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MetricDefinition {
    pub id: MetricId,
    pub otel_name: &'static str,
    pub prometheus_name: &'static str,
    pub description: &'static str,
    pub unit: Option<&'static str>,
    pub label_names: &'static [&'static str],
    pub prometheus_label_names: &'static [&'static str],
    pub kind: MetricKind,
    pub buckets: Option<&'static [f64]>,
    pub prewarm_label_values: &'static [&'static [&'static str]],
}

pub const METRIC_DEFINITIONS: &[MetricDefinition] = &[
    MetricDefinition {
        id: MetricId::MsgConsumeLatency,
        otel_name: "chronos.message.consume.duration",
        prometheus_name: "msg_consume_latency",
        description: "Duration of handle_message() in message_receiver.",
        unit: Some("s"),
        label_names: &["destination", "status"],
        prometheus_label_names: &["destination", "status"],
        kind: MetricKind::Histogram,
        buckets: Some(&[0.001, 0.002, 0.004, 0.008, 0.016, 0.032, 0.064, 0.128, 0.256, 0.512, 1.024, 2.048]),
        prewarm_label_values: &[&["kafka", "pass"], &["kafka", "fail"], &["postgres", "pass"], &["postgres", "fail"]],
    },
    MetricDefinition {
        id: MetricId::MsgJitter,
        otel_name: "chronos.message.jitter",
        prometheus_name: "msg_jitter",
        description: "Difference between actual publish time and client-requested deadline.",
        unit: Some("s"),
        label_names: &[],
        prometheus_label_names: &[],
        kind: MetricKind::Histogram,
        buckets: Some(&[0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]),
        prewarm_label_values: &[],
    },
    MetricDefinition {
        id: MetricId::MsgProcessLatency,
        otel_name: "chronos.message.process.duration",
        prometheus_name: "msg_process_latency",
        description: "Duration of processor_message_ready() loop in message_processor.",
        unit: Some("s"),
        label_names: &["returned", "status"],
        prometheus_label_names: &["returned", "status"],
        kind: MetricKind::Histogram,
        buckets: Some(&[0.001, 0.002, 0.004, 0.008, 0.016, 0.032, 0.064, 0.128, 0.256, 0.512, 1.024, 2.048]),
        prewarm_label_values: &[&["true", "pass"], &["true", "fail"], &["false", "pass"], &["false", "fail"]],
    },
    MetricDefinition {
        id: MetricId::MsgReset,
        otel_name: "chronos.message.reset",
        prometheus_name: "msg_reset",
        description: "Number of records reset by reset_to_init_db() in the monitor task.",
        unit: Some("{message}"),
        label_names: &[],
        prometheus_label_names: &[],
        kind: MetricKind::Counter,
        buckets: None,
        prewarm_label_values: &[],
    },
    MetricDefinition {
        id: MetricId::MsgWaitTime,
        otel_name: "chronos.message.wait.duration",
        prometheus_name: "msg_wait_time",
        description: "Time a message spent in the Kafka input queue before processing.",
        unit: Some("s"),
        label_names: &[],
        prometheus_label_names: &[],
        kind: MetricKind::Histogram,
        buckets: Some(&[0.1, 0.2, 0.4, 0.8, 1.6, 3.2, 6.4, 12.8, 25.6, 51.2, 102.4, 204.8, 409.6, 819.2]),
        prewarm_label_values: &[],
    },
];
