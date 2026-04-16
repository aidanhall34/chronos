use prometheus::{exponential_buckets, histogram_opts, opts, Counter, Histogram, HistogramVec, Registry};

/// All Prometheus metrics for Chronos.
/// Uses a per-instance Registry so tests can create isolated instances
/// without "already registered" collisions.
pub struct ChronosMetrics {
    pub registry: Registry,
    /// Duration of handle_message() in message_receiver. Labels: [destination, status]
    ///   destination = "kafka" | "postgres"
    ///   status = "pass" | "fail"
    pub msg_consume_latency: HistogramVec,
    /// Duration of processor_message_ready() loop in message_processor. Labels: [returned, status]
    ///   returned = "true" (no rows, loop returned early) | "false" (rows processed)
    ///   status = "pass" | "fail"
    pub msg_process_latency: HistogramVec,
    /// Time a message spent in the Kafka input queue before being processed.
    pub msg_wait_time: Histogram,
    /// Difference between actual publish time and client-requested deadline (jitter).
    /// Includes an explicit 0.5s bucket matching the 500ms SLA.
    pub msg_jitter: Histogram,
    /// Number of records reset by reset_to_init_db() (the monitor task).
    pub msg_reset: Counter,
}

impl std::fmt::Debug for ChronosMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChronosMetrics").finish()
    }
}

impl ChronosMetrics {
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();

        let consume_buckets = exponential_buckets(0.001, 2.0, 12)?;
        let msg_consume_latency = HistogramVec::new(
            histogram_opts!("msg_consume_latency", "Duration of handle_message() in message_receiver", consume_buckets),
            &["destination", "status"],
        )?;
        registry.register(Box::new(msg_consume_latency.clone()))?;
        // Pre-warm all label combinations so the metric family always appears in gather()
        // output from startup — HistogramVec is omitted from gather() until at least one
        // label combination has been touched.
        for destination in &["kafka", "postgres"] {
            for status in &["pass", "fail"] {
                msg_consume_latency.get_metric_with_label_values(&[destination, status])?;
            }
        }

        let process_buckets = exponential_buckets(0.001, 2.0, 12)?;
        let msg_process_latency = HistogramVec::new(
            histogram_opts!(
                "msg_process_latency",
                "Duration of processor_message_ready() loop in message_processor",
                process_buckets
            ),
            &["returned", "status"],
        )?;
        registry.register(Box::new(msg_process_latency.clone()))?;
        // Pre-warm all label combinations for the same reason as msg_consume_latency above.
        for returned in &["true", "false"] {
            for status in &["pass", "fail"] {
                msg_process_latency.get_metric_with_label_values(&[returned, status])?;
            }
        }

        let wait_buckets = exponential_buckets(0.1, 2.0, 14)?;
        let msg_wait_time = Histogram::with_opts(histogram_opts!(
            "msg_wait_time",
            "Time a message spent in the Kafka input queue before processing",
            wait_buckets
        ))?;
        registry.register(Box::new(msg_wait_time.clone()))?;

        // Custom buckets with explicit 0.5s boundary for the 500ms SLA
        let jitter_buckets = vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0];
        let msg_jitter = Histogram::with_opts(histogram_opts!(
            "msg_jitter",
            "Difference between actual publish time and client-requested deadline",
            jitter_buckets
        ))?;
        registry.register(Box::new(msg_jitter.clone()))?;

        let msg_reset = Counter::with_opts(opts!("msg_reset", "Number of records reset by reset_to_init_db()"))?;
        registry.register(Box::new(msg_reset.clone()))?;

        Ok(ChronosMetrics {
            registry,
            msg_consume_latency,
            msg_process_latency,
            msg_wait_time,
            msg_jitter,
            msg_reset,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prometheus::{Encoder, TextEncoder};

    #[test]
    fn test_metrics_registry_creates_successfully() {
        assert!(ChronosMetrics::new().is_ok());
    }

    #[test]
    fn test_msg_consume_latency_records_observation() {
        let metrics = ChronosMetrics::new().unwrap();
        metrics
            .msg_consume_latency
            .get_metric_with_label_values(&["kafka", "pass"])
            .unwrap()
            .observe(0.05);

        let families = metrics.registry.gather();
        let fam = families.iter().find(|f| f.get_name() == "msg_consume_latency").unwrap();
        // With pre-warming there are 4 entries; find the kafka/pass one by its labels.
        let kafka_pass = fam.get_metric().iter().find(|m| {
            m.get_label().iter().any(|l| l.get_name() == "destination" && l.get_value() == "kafka")
                && m.get_label().iter().any(|l| l.get_name() == "status" && l.get_value() == "pass")
        });
        assert!(kafka_pass.is_some(), "kafka/pass label combination must exist");
        let sample_sum = kafka_pass.unwrap().get_histogram().get_sample_sum();
        assert!((sample_sum - 0.05).abs() < 1e-9);
    }

    #[test]
    fn test_msg_jitter_has_500ms_bucket() {
        let metrics = ChronosMetrics::new().unwrap();
        // Observe a value just below 500ms
        metrics.msg_jitter.observe(0.499);

        let families = metrics.registry.gather();
        let fam = families.iter().find(|f| f.get_name() == "msg_jitter").unwrap();
        let histogram = fam.get_metric()[0].get_histogram();

        let bucket_500ms = histogram.get_bucket().iter().find(|b| (b.get_upper_bound() - 0.5).abs() < 1e-9);
        assert!(bucket_500ms.is_some(), "0.5s bucket must exist in msg_jitter");
        assert_eq!(
            bucket_500ms.unwrap().get_cumulative_count(),
            1,
            "0.499s observation must be counted in the <=0.5s bucket"
        );
    }

    #[test]
    fn test_msg_reset_increments_correctly() {
        let metrics = ChronosMetrics::new().unwrap();
        metrics.msg_reset.inc_by(3.0);
        metrics.msg_reset.inc_by(2.0);

        let families = metrics.registry.gather();
        let fam = families.iter().find(|f| f.get_name() == "msg_reset").unwrap();
        let value = fam.get_metric()[0].get_counter().get_value();
        assert!((value - 5.0).abs() < 1e-9);
    }

    #[test]
    fn test_msg_wait_time_records_observation() {
        let metrics = ChronosMetrics::new().unwrap();
        metrics.msg_wait_time.observe(1.5);

        let families = metrics.registry.gather();
        let fam = families.iter().find(|f| f.get_name() == "msg_wait_time").unwrap();
        let sample_count = fam.get_metric()[0].get_histogram().get_sample_count();
        assert_eq!(sample_count, 1);
    }

    #[test]
    fn test_msg_process_latency_label_values() {
        let metrics = ChronosMetrics::new().unwrap();
        metrics
            .msg_process_latency
            .get_metric_with_label_values(&["true", "pass"])
            .unwrap()
            .observe(0.01);
        metrics
            .msg_process_latency
            .get_metric_with_label_values(&["false", "pass"])
            .unwrap()
            .observe(0.05);
        metrics
            .msg_process_latency
            .get_metric_with_label_values(&["false", "fail"])
            .unwrap()
            .observe(0.1);

        let families = metrics.registry.gather();
        let fam = families.iter().find(|f| f.get_name() == "msg_process_latency").unwrap();
        // 3 explicit observations + pre-warming fills all 4 combos; de-dup means 4 entries.
        assert_eq!(fam.get_metric().len(), 4);
    }

    #[test]
    fn test_metrics_text_encode_produces_output() {
        let metrics = ChronosMetrics::new().unwrap();
        // All metrics should appear without any manual observations because HistogramVec
        // combos are pre-warmed in new(). Scalar histograms and counters always appear.
        let encoder = TextEncoder::new();
        let mut buffer = Vec::new();
        encoder.encode(&metrics.registry.gather(), &mut buffer).unwrap();
        let output = String::from_utf8(buffer).unwrap();

        assert!(output.contains("msg_reset"));
        assert!(output.contains("msg_jitter"));
        assert!(output.contains("msg_wait_time"));
        assert!(output.contains("msg_consume_latency"));
        assert!(output.contains("msg_process_latency"));
    }

    /// All 5 metric families must be present in a freshly constructed registry even
    /// before any messages are processed (i.e., with zero observations).
    #[test]
    fn test_all_metrics_present_in_fresh_registry() {
        let metrics = ChronosMetrics::new().unwrap();
        let encoder = TextEncoder::new();
        let mut buffer = Vec::new();
        encoder.encode(&metrics.registry.gather(), &mut buffer).unwrap();
        let output = String::from_utf8(buffer).unwrap();

        for name in &["msg_consume_latency", "msg_process_latency", "msg_wait_time", "msg_jitter", "msg_reset"] {
            assert!(
                output.contains(&format!("# HELP {}", name)),
                "metric {} must appear in fresh registry output",
                name
            );
        }
    }

    /// msg_consume_latency must have exactly 4 pre-initialized label combinations
    /// (kafka/postgres × pass/fail) so it is always present in the scrape output.
    #[test]
    fn test_consume_latency_all_label_combos_initialized() {
        let metrics = ChronosMetrics::new().unwrap();
        let families = metrics.registry.gather();
        let fam = families
            .iter()
            .find(|f| f.get_name() == "msg_consume_latency")
            .expect("msg_consume_latency must be present in a fresh registry");
        assert_eq!(fam.get_metric().len(), 4, "expected 4 pre-warmed label combos (kafka/postgres × pass/fail)");
    }

    /// msg_process_latency must have exactly 4 pre-initialized label combinations
    /// (true/false × pass/fail) so it is always present in the scrape output.
    #[test]
    fn test_process_latency_all_label_combos_initialized() {
        let metrics = ChronosMetrics::new().unwrap();
        let families = metrics.registry.gather();
        let fam = families
            .iter()
            .find(|f| f.get_name() == "msg_process_latency")
            .expect("msg_process_latency must be present in a fresh registry");
        assert_eq!(fam.get_metric().len(), 4, "expected 4 pre-warmed label combos (true/false × pass/fail)");
    }
}
