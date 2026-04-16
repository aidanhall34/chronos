use crate::kafka::consumer::KafkaConsumer;
use crate::kafka::producer::KafkaProducer;
use crate::message_processor::MessageProcessor;
use crate::message_receiver::MessageReceiver;
use crate::metrics::server::run_metrics_server;
use crate::metrics::ChronosMetrics;
use crate::monitor::FailureDetector;
use crate::postgres::pg::Pg;
use crate::utils::config::ChronosConfig;
use std::fs::{create_dir, read, write};
use std::sync::Arc;

pub struct Runner {
    pub consumer: Arc<KafkaConsumer>,
    pub producer: Arc<KafkaProducer>,
    pub data_store: Arc<Pg>,
    pub metrics: Arc<ChronosMetrics>,
}

impl Runner {
    pub async fn run(&self) {
        let monitor_ds = Arc::clone(&self.data_store);
        let monitor_metrics = Arc::clone(&self.metrics);

        let process_ds = Arc::clone(&self.data_store);
        let process_producer = self.producer.clone();
        let process_metrics = Arc::clone(&self.metrics);

        let receiver_ds = Arc::clone(&self.data_store);
        let receiver_prod = self.producer.clone();
        let receiver_consumer = self.consumer.clone();
        let receiver_metrics = Arc::clone(&self.metrics);

        let metrics_port = ChronosConfig::from_env().metrics_port;
        let metrics_for_server = Arc::clone(&self.metrics);

        // Spawn metrics server as an independent background task.
        // A failure here is logged but does not stop the processing tasks.
        tokio::task::spawn(async move {
            run_metrics_server(metrics_for_server, metrics_port).await;
        });

        let monitor_handler = tokio::task::spawn(async {
            let monitor = FailureDetector {
                data_store: monitor_ds,
                metrics: monitor_metrics,
            };
            monitor.run().await;
        });
        let message_processor_handler = tokio::task::spawn(async {
            let message_processor = MessageProcessor {
                producer: process_producer,
                data_store: process_ds,
                metrics: process_metrics,
            };
            message_processor.run().await;
        });
        let message_receiver_handler = tokio::task::spawn(async {
            let message_receiver = MessageReceiver {
                consumer: receiver_consumer,
                producer: receiver_prod,
                data_store: receiver_ds,
                metrics: receiver_metrics,
            };

            message_receiver.run().await;
        });

        // check if healthcheck file exists in healthcheck dir
        let healthcheck_file = std::env::var("HEALTHCHECK_FILE").unwrap_or_else(|_| "healthcheck/chronos_healthcheck".to_string());
        let healthcheck_file_exists = read(&healthcheck_file).is_ok();
        if healthcheck_file_exists {
            log::info!("healthcheck file exists");
            let write_resp = write(&healthcheck_file, b"1");
            if write_resp.is_err() {
                log::error!("error while writing to healthcheck file {:?}", write_resp);
            }
        } else if create_dir("healthcheck").is_ok() {
            let write_resp = write(&healthcheck_file, b"1");
            if write_resp.is_err() {
                log::error!("error while writing to healthcheck file {:?}", write_resp);
            }
        }
        let future_tuple = futures::future::try_join3(monitor_handler, message_processor_handler, message_receiver_handler).await;
        if future_tuple.is_err() {
            log::error!("Chronos Stopping all threads {:?}", future_tuple);
            let write_resp = write(&healthcheck_file, b"0");
            if write_resp.is_err() {
                log::error!("error while writing to healthcheck file {:?}", write_resp);
            }
        }
    }
}
