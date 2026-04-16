use std::time::Duration;

use crate::kafka::errors::KafkaAdapterError;
use log::info;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::BorrowedMessage;

use super::config::KafkaConfig;

use tracing::{trace, warn};

// Kafka Consumer Client
pub struct KafkaConsumer {
    pub consumer: StreamConsumer,
    pub topic: String,
}

impl KafkaConsumer {
    pub fn new(config: &KafkaConfig) -> Self {
        // let consumer = config.build_consumer_config().create().expect("Failed to create consumer");
        let consumer = match config.build_consumer_config().create() {
            Ok(consumer) => {
                //TODO add more information to the trace, which broker, which topic, etc
                trace!("connected to kafka");
                consumer
            }
            Err(e) => {
                // Add error trace to the span
                warn!("error creating consumer {:?}", e);

                log::error!("error creating consumer {:?}", e);
                //retry
                log::info!("retrying in 5 seconds");
                std::thread::sleep(Duration::from_secs(5));
                loop {
                    match config.build_consumer_config().create() {
                        Ok(consumer) => {
                            log::info!("connected to kafka");
                            break consumer;
                        }
                        Err(e) => {
                            log::error!("error creating consumer {:?}", e);
                            //retry
                            log::info!("retrying in 5 seconds");
                            std::thread::sleep(Duration::from_secs(5));
                        }
                    }
                }
            }
        };

        let topic = config.in_topic.clone();
        Self { consumer, topic }
    }

    pub(crate) async fn subscribe(&self) {
        match &self.consumer.subscribe(&[&self.topic]) {
            Ok(_) => {
                info!("subscribed to topic {}", &self.topic);
            }
            Err(e) => {
                log::error!("error while subscribing to topic {}", e);
                //add retry logic
            }
        };
    }

    pub(crate) async fn kafka_consume_message(&self) -> Result<BorrowedMessage, KafkaAdapterError> {
        self.consumer.recv().await.map_err(KafkaAdapterError::ReceiveMessage)
    }
}
