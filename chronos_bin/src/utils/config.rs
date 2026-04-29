#[derive(Debug, Clone)]
pub struct ChronosConfig {
    // pub random_delay: u64,
    pub monitor_db_poll: u64,
    pub processor_db_poll: u64,
    pub time_advance: u64,
    pub fail_detect_interval: u64,
    pub metrics_host: String,
    pub metrics_port: u16,
}

impl ChronosConfig {
    pub fn from_env() -> ChronosConfig {
        ChronosConfig {
            // random_delay: env_var!("RANDOMNESS_DELAY").parse().unwrap(),
            monitor_db_poll: std::env::var("MONITOR_DB_POLL").unwrap_or_else(|_| 5.to_string()).parse().unwrap_or(5),
            processor_db_poll: std::env::var("PROCESSOR_DB_POLL").unwrap_or_else(|_| 5.to_string()).parse().unwrap_or(5),
            time_advance: std::env::var("TIMING_ADVANCE").unwrap_or_else(|_| 0.to_string()).parse().unwrap_or(0),
            fail_detect_interval: std::env::var("FAIL_DETECT_INTERVAL").unwrap_or_else(|_| 10.to_string()).parse().unwrap_or(10),
            metrics_host: std::env::var("OTEL_EXPORTER_PROMETHEUS_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            metrics_port: std::env::var("OTEL_EXPORTER_PROMETHEUS_PORT")
                .or_else(|_| std::env::var("METRICS_PORT"))
                .unwrap_or_else(|_| "9090".to_string())
                .parse()
                .unwrap_or(9090),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ChronosConfig;
    use serial_test::serial;

    fn remove_metrics_env() {
        std::env::remove_var("OTEL_EXPORTER_PROMETHEUS_HOST");
        std::env::remove_var("OTEL_EXPORTER_PROMETHEUS_PORT");
        std::env::remove_var("METRICS_PORT");
    }

    #[test]
    #[serial]
    fn prometheus_spec_env_overrides_metrics_binding() {
        remove_metrics_env();
        std::env::set_var("OTEL_EXPORTER_PROMETHEUS_HOST", "127.0.0.1");
        std::env::set_var("OTEL_EXPORTER_PROMETHEUS_PORT", "9464");
        std::env::set_var("METRICS_PORT", "9090");

        let config = ChronosConfig::from_env();

        assert_eq!(config.metrics_host, "127.0.0.1");
        assert_eq!(config.metrics_port, 9464);
        remove_metrics_env();
    }

    #[test]
    #[serial]
    fn metrics_port_remains_backward_compatible_fallback() {
        remove_metrics_env();
        std::env::set_var("METRICS_PORT", "9091");

        let config = ChronosConfig::from_env();

        assert_eq!(config.metrics_host, "0.0.0.0");
        assert_eq!(config.metrics_port, 9091);
        remove_metrics_env();
    }
}
