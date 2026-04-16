use std::collections::HashMap;
use std::time::Duration;

use crate::utils::util::into_headers;
use crate::{kafka::errors::KafkaAdapterError, utils::util::CHRONOS_ID};
use rdkafka::producer::{FutureProducer, FutureRecord};

use super::config::KafkaConfig;

use tracing::instrument;

// Kafka Producer
// #[derive(Clone)]
pub struct KafkaProducer {
    producer: FutureProducer,
    topic: String,
}

impl KafkaProducer {
    pub fn new(config: &KafkaConfig) -> Self {
        // rdlibkafka goes infinitely trying to connect to kafka broker
        let producer = config.build_producer_config().create().expect("Failed to create producer");
        let topic = config.out_topic.to_owned();

        Self { producer, topic }
    }
    #[instrument(skip_all, fields(topic = %self.topic))]
    pub async fn kafka_publish(&self, message: String, headers: Option<HashMap<String, String>>, key: String) -> Result<String, KafkaAdapterError> {
        // Only because never expecting wrong headers to reach here
        let unwrap_header = &headers.unwrap_or_default();

        let o_header = into_headers(unwrap_header);
        // println!("headers {:?}", o_header);
        // println!("headers {:?} headers--{:?}", &headers["chronosId)"].to_string(), &headers["chronosDeadline)"].to_string());

        let _delivery_status = self
            .producer
            .send(
                FutureRecord::to(self.topic.as_str())
                    .payload(message.as_str())
                    .key(key.as_str())
                    .headers(o_header),
                Duration::from_secs(0),
            )
            .await
            .map_err(|(kafka_error, _record)| KafkaAdapterError::PublishMessage(kafka_error, "message publishing failed".to_string()))?;
        Ok(unwrap_header[CHRONOS_ID].to_string())
    }
}
