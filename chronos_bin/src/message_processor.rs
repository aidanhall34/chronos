use crate::kafka::producer::KafkaProducer;
use crate::metrics::ChronosMetrics;
use crate::postgres::pg::{GetReady, Pg, TableRow};
use crate::utils::config::ChronosConfig;
use crate::utils::delay_controller::DelayController;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio_postgres::Row;
use uuid::Uuid;

pub struct MessageProcessor {
    pub(crate) data_store: Arc<Pg>,
    pub(crate) producer: Arc<KafkaProducer>,
    pub(crate) metrics: Arc<ChronosMetrics>,
}

impl MessageProcessor {
    fn assign_node_id() -> Uuid {
        let node_id: Uuid = match std::env::var("NODE_ID") {
            Ok(val) => Uuid::parse_str(&val).unwrap_or_else(|_e| {
                let uuid = uuid::Uuid::new_v4();
                log::info!("NODE_ID not found in env assigning {}", uuid);
                uuid
            }),
            Err(_e) => {
                log::info!("NODE_ID not found in env");
                uuid::Uuid::new_v4()
            }
        };
        node_id
    }

    #[tracing::instrument(skip_all, fields(correlationId))]
    async fn prepare_to_publish(&self, row: Row) -> Result<String, String> {
        let updated_row = TableRow {
            id: row.get("id"),
            deadline: row.get("deadline"),
            readied_at: row.get("readied_at"),
            readied_by: row.get("readied_by"),
            message_headers: row.get("message_headers"),
            message_key: row.get("message_key"),
            message_value: row.get("message_value"),
        };
        let mut headers: HashMap<String, String> = match serde_json::from_str(&updated_row.message_headers.to_string()) {
            Ok(t) => t,
            Err(_e) => {
                log::error!("error occurred while parsing");
                return Err("error occurred while parsing headers field".to_string());
            }
        };

        // Capture deadline before updated_row fields are moved into the publish call.
        let deadline = updated_row.deadline;

        let readied_by_column = Some(updated_row.readied_by.to_string());
        tracing::Span::current().record("correlationId", &readied_by_column);

        match readied_by_column {
            Some(id) => {
                headers.insert("readied_by".to_string(), id);
                if let Ok(id) = self
                    .producer
                    .kafka_publish(updated_row.message_value.to_string(), Some(headers), updated_row.message_key.to_string())
                    .await
                {
                    // msg_jitter: difference between actual publish time and client-requested deadline.
                    // Floored at 0 to guard against clock skew producing negative jitter.
                    let jitter_secs = (Utc::now() - deadline).num_milliseconds().max(0) as f64 / 1000.0;
                    self.metrics.msg_jitter.observe(jitter_secs);
                    Ok(id)
                } else {
                    Err("error occurred while publishing".to_string())
                }
            }

            None => {
                log::error!("Error: readied_by not found in db row {:?}", updated_row);
                Err("error occurred while publishing".to_string())
            }
        }
    }

    #[tracing::instrument(skip_all, fields(deleted_ids))]
    async fn delete_fired_records_from_db(&self, ids: &Vec<String>) {
        //retry loop
        let max_retry_count = 3;
        let mut retry_count = 0;
        while let Err(outcome_error) = &self.data_store.delete_fired(ids).await {
            log::error!("Error: error occurred in message processor {}", outcome_error);
            retry_count += 1;
            if retry_count == max_retry_count {
                log::error!("Error: max retry count {} reached by node {:?} for deleting fired ids ", max_retry_count, ids);
                break;
            }
        }
    }

    /// Returns `(returned, status)` where:
    ///   - `returned = true`  means the loop exited early (no rows ready to fire)
    ///   - `returned = false` means rows were processed (or a terminal error occurred)
    ///   - `status = "pass"` on success, `"fail"` on unrecoverable error
    #[tracing::instrument(skip_all)]
    async fn processor_message_ready(&self, node_id: Uuid) -> (bool, &'static str) {
        loop {
            log::debug!("retry loop");
            let max_retry_count = 3;
            let mut retry_count = 0;

            let deadline = Utc::now() - Duration::from_secs(ChronosConfig::from_env().time_advance);

            let param = GetReady {
                readied_at: deadline,
                readied_by: node_id,
                deadline,
            };

            let readied_by_column: Option<String> = None;
            let resp: Result<Vec<Row>, String> = self.data_store.ready_to_fire_db(&param).await;
            match resp {
                Ok(ready_to_publish_rows) => {
                    if ready_to_publish_rows.is_empty() {
                        log::debug!("no rows ready to fire for deadline {}", deadline);
                        return (true, "pass");
                    } else {
                        let publish_futures = ready_to_publish_rows.into_iter().map(|row| self.prepare_to_publish(row));

                        let results = futures::future::join_all(publish_futures).await;

                        let ids: Vec<String> = results.into_iter().filter_map(|result| result.ok()).collect();

                        if !ids.is_empty() {
                            let _ = self.delete_fired_records_from_db(&ids).await;
                            log::debug!("number of rows published successfully and deleted from DB {}", ids.len());
                            return (false, "pass");
                        }
                    }
                }
                Err(e) => {
                    if e.contains("could not serialize access due to concurrent update") && retry_count < max_retry_count {
                        retry_count += 1;
                        if retry_count == max_retry_count {
                            log::error!("Error: max retry count {} reached by node {:?} for row ", max_retry_count, readied_by_column);
                            return (false, "fail");
                        }
                    }
                    log::error!("Error: error occurred in message processor while publishing {}", e);
                }
            }
        }
    }

    pub async fn run(&self) {
        log::info!("MessageProcessor ON!");

        //Get UUID for the node that deployed this thread
        let node_id = Self::assign_node_id();

        log::info!("node_id {}", node_id);
        let mut delay_controller = DelayController::new(100);
        loop {
            log::debug!("MessageProcessor loop");
            tokio::time::sleep(Duration::from_millis(10)).await;

            // msg_process_latency: time the full processor_message_ready() call.
            let timer = std::time::Instant::now();
            let (returned, status) = self.processor_message_ready(node_id).await;
            let elapsed = timer.elapsed().as_secs_f64();
            if let Ok(obs) = self.metrics.msg_process_latency.get_metric_with_label_values(&[&returned.to_string(), status]) {
                obs.observe(elapsed);
            } else {
                log::error!("metrics: failed to observe msg_process_latency");
            }

            delay_controller.sleep().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::metrics::ChronosMetrics;

    #[test]
    fn test_jitter_calculation_positive() {
        use chrono::{Duration, Utc};
        let deadline = Utc::now() - Duration::milliseconds(300);
        let jitter_ms = (Utc::now() - deadline).num_milliseconds().max(0);
        assert!(jitter_ms >= 300, "jitter should be at least 300ms when deadline was 300ms ago");
    }

    #[test]
    fn test_jitter_below_500ms_within_sla() {
        let metrics = ChronosMetrics::new().unwrap();
        // A 300ms jitter is within the 500ms SLA — must land in the <=0.5s bucket
        metrics.msg_jitter.observe(0.3);
        let families = metrics.registry.gather();
        let fam = families.iter().find(|f| f.get_name() == "msg_jitter").unwrap();
        let hist = fam.get_metric()[0].get_histogram();
        let bucket_500 = hist.get_bucket().iter().find(|b| (b.get_upper_bound() - 0.5).abs() < 1e-9).unwrap();
        assert_eq!(bucket_500.get_cumulative_count(), 1, "300ms jitter must be counted in the <=500ms bucket");
    }
}
