use chrono::{DateTime, Utc};
use serde_json::json;
use tracing::instrument;

use crate::kafka::consumer::KafkaConsumer;
use crate::kafka::producer::KafkaProducer;
use crate::postgres::pg::{Pg, TableInsertRow};
use crate::utils::util::{get_message_key, get_payload_utf8, required_headers, CHRONOS_ID, DEADLINE};
use rdkafka::message::BorrowedMessage;
use std::{collections::HashMap, str::FromStr, sync::Arc};

pub struct MessageReceiver {
    pub(crate) consumer: Arc<KafkaConsumer>,
    pub(crate) producer: Arc<KafkaProducer>,
    pub(crate) data_store: Arc<Pg>,
}

impl MessageReceiver {
    #[instrument(skip_all, fields(correlationId))]
    async fn insert_into_db(
        &self,
        new_message: &BorrowedMessage<'_>,
        reqd_headers: HashMap<String, String>,
        message_deadline: DateTime<Utc>,
    ) -> Option<String> {
        let max_retry_count = 3;
        let mut retry_count = 0;
        //retry loop
        loop {
            if let Some(payload) = get_payload_utf8(new_message) {
                if let Ok(message_value) = &serde_json::from_slice(payload) {
                    if let Some(message_key) = get_message_key(new_message) {
                        let params = TableInsertRow {
                            id: &reqd_headers[CHRONOS_ID],
                            deadline: message_deadline,
                            message_headers: &json!(&reqd_headers),
                            message_key: message_key.as_str(),
                            message_value,
                        };

                        if let Err(e) = self.data_store.insert_to_delay_db(&params).await {
                            log::error!("insert to delay failed {}", e);
                            retry_count += 1;
                            if retry_count == max_retry_count {
                                return Some("max retry count reached for insert to delay query".to_string());
                            }
                            continue;
                        }
                        tracing::Span::current().record("correlationId", &message_key);
                    }

                    log::debug!("Message publish success {:?}", new_message);
                    return None;
                } else {
                    return Some("json conversion of payload failed".to_string());
                }
            } else {
                return Some("message payload is not utf8 encoded".to_string());
            }
        }
    }

    #[instrument(skip_all, fields(correlationId))]
    async fn prepare_and_publish(&self, message: &BorrowedMessage<'_>, reqd_headers: HashMap<String, String>) -> Option<String> {
        match get_payload_utf8(message) {
            Some(string_payload) => {
                if let Some(message_key) = get_message_key(message) {
                    let string_payload = String::from_utf8_lossy(string_payload).to_string();
                    tracing::Span::current().record("correlationId", &message_key);
                    if let Err(e) = &self.producer.kafka_publish(string_payload, Some(reqd_headers.clone()), message_key).await {
                        return Some(format!("publish failed for received message {:?} with error :: {}", message, e));
                    }
                } else {
                    return Some("message key not found".to_string());
                }
            }
            None => return None,
        };
        None
    }

    #[tracing::instrument(name = "receiver_handle_message", skip_all, fields(correlationId, error))]
    pub async fn handle_message(&self, message: &BorrowedMessage<'_>) {
        let new_message = &message;
        if let Some(reqd_headers) = required_headers(new_message) {
            tracing::Span::current().record("correlationId", &reqd_headers[CHRONOS_ID]);
            if let Ok(message_deadline) = DateTime::<Utc>::from_str(&reqd_headers[DEADLINE]) {
                if message_deadline <= Utc::now() {
                    if let Some(err) = self.prepare_and_publish(new_message, reqd_headers).await {
                        log::error!("{}", err);
                        tracing::Span::current().record("error", &err);
                    }
                } else if let Some(err_string) = self.insert_into_db(new_message, reqd_headers, message_deadline).await {
                    log::error!("{}", err_string);
                    tracing::Span::current().record("error", &err_string);
                }
            }
        } else {
            log::warn!("message receiver: required headers not found");
        }
    }

    pub async fn run(&self) {
        log::info!("MessageReceiver ON!");
        let _ = &self.consumer.subscribe().await;
        loop {
            match &self.consumer.kafka_consume_message().await {
                Ok(message) => {
                    self.handle_message(message).await;
                }
                Err(e) => {
                    log::error!("error while consuming message {:?}", e);
                }
            }
            // if let Ok(message) = &self.consumer.kafka_consume_message().await {
            //     self.handle_message(message).await;
            // }
        }
    }
}
