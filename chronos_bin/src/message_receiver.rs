use chrono::{DateTime, Utc};
use rdkafka::message::BorrowedMessage;
use rdkafka::Message;
use serde_json::json;
use std::{collections::HashMap, str::FromStr, sync::Arc};
use tracing::instrument;

use crate::kafka::consumer::KafkaConsumer;
use crate::kafka::producer::KafkaProducer;
use crate::metrics::ChronosMetrics;
use crate::postgres::pg::{Pg, TableInsertRow};
use crate::utils::util::{get_message_key, get_payload_utf8, required_headers, CHRONOS_ID, DEADLINE};

pub struct MessageReceiver {
    pub(crate) consumer: Arc<KafkaConsumer>,
    pub(crate) producer: Arc<KafkaProducer>,
    pub(crate) data_store: Arc<Pg>,
    pub(crate) metrics: Arc<ChronosMetrics>,
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
        // msg_wait_time: record how long the message waited in the Kafka input queue.
        // Uses the Kafka-assigned message timestamp; guards against clock skew with max(0).
        if let Some(kafka_ts_ms) = message.timestamp().to_millis() {
            let wait_secs = (Utc::now().timestamp_millis() - kafka_ts_ms).max(0) as f64 / 1000.0;
            self.metrics.msg_wait_time.observe(wait_secs);
        }

        let timer = std::time::Instant::now();
        let mut destination = "unknown";
        let mut status = "pass";

        let new_message = &message;
        if let Some(reqd_headers) = required_headers(new_message) {
            tracing::Span::current().record("correlationId", &reqd_headers[CHRONOS_ID]);
            if let Ok(message_deadline) = DateTime::<Utc>::from_str(&reqd_headers[DEADLINE]) {
                if message_deadline <= Utc::now() {
                    destination = "kafka";
                    if let Some(err) = self.prepare_and_publish(new_message, reqd_headers).await {
                        status = "fail";
                        log::error!("{}", err);
                        tracing::Span::current().record("error", &err);
                    }
                } else {
                    destination = "postgres";
                    if let Some(err_string) = self.insert_into_db(new_message, reqd_headers, message_deadline).await {
                        status = "fail";
                        log::error!("{}", err_string);
                        tracing::Span::current().record("error", &err_string);
                    }
                }
            }
        }

        // msg_consume_latency: only record when destination was determined (valid message headers).
        if destination != "unknown" {
            let elapsed = timer.elapsed().as_secs_f64();
            if let Ok(obs) = self.metrics.msg_consume_latency.get_metric_with_label_values(&[destination, status]) {
                obs.observe(elapsed);
            } else {
                log::error!("metrics: failed to observe msg_consume_latency");
            }
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
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_wait_time_calculation_non_negative() {
        let kafka_ts_ms: i64 = 1_700_000_000_000;
        let now_ms: i64 = kafka_ts_ms + 5_000;
        let wait_secs = (now_ms - kafka_ts_ms).max(0) as f64 / 1000.0;
        assert!((wait_secs - 5.0).abs() < 1e-9);
    }

    #[test]
    fn test_wait_time_calculation_clock_skew() {
        // Simulates a future Kafka timestamp (clock skew) — should floor to 0.0
        let kafka_ts_ms: i64 = 9_999_999_999_999;
        let now_ms: i64 = 1_700_000_000_000;
        let wait_secs = (now_ms - kafka_ts_ms).max(0) as f64 / 1000.0;
        assert_eq!(wait_secs, 0.0);
    }
}
