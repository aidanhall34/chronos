use crate::kafka::producer::KafkaProducer;
use crate::postgres::pg::{GetReady, Pg, TableRow};
use crate::utils::config::ChronosConfig;
use crate::utils::delay_controller::DelayController;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio_postgres::Row;
use uuid::Uuid;

use tracing::event;

pub struct MessageProcessor {
    pub(crate) data_store: Arc<Pg>,
    pub(crate) producer: Arc<KafkaProducer>,
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

    #[tracing::instrument(name = "prepare_to_publish", skip_all, fields(correlationId))]
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
        let method_name = "delete_fired_records_from_db";
        let max_retry_count = 3;
        let mut retry_count = 0;
        while let Err(outcome_error) = &self.data_store.delete_fired(ids).await {
            log::error!("{}: error occurred in message processor {}", method_name, outcome_error);
            retry_count += 1;
            if retry_count == max_retry_count {
                log::error!(
                    "{}: max retry count {} reached by node {:?} for deleting fired ids ",
                    method_name,
                    max_retry_count,
                    ids
                );
                break;
            }
        }
    }

    async fn retry_loop(&self, param: &GetReady) -> Result<Vec<Row>, String> {
        let max_retry_count = 3;
        let mut retry_count = 0;
        loop {
            match self.data_store.ready_to_fire_db(param).await {
                Ok(rows) => return Ok(rows),
                Err(e) => {
                    retry_count += 1;
                    if retry_count >= max_retry_count {
                        return Err(e);
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(20 * retry_count as u64)).await;
                }
            }
        }
    }

    #[tracing::instrument(name = "processor_message_ready", skip_all, fields(correlationId))]
    async fn processor_message_ready(&self, node_id: Uuid) {
        loop {
            let method_name = "processor_message_ready";

            let deadline = Utc::now() - Duration::from_secs(ChronosConfig::from_env().time_advance);

            let param = GetReady {
                readied_at: deadline,
                readied_by: node_id,
                deadline,
                // limit: 1000,
                // order: "asc",
            };

            let resp: Result<Vec<Row>, String> = self.retry_loop(&param).await;
            match resp {
                Ok(ready_to_publish_rows) => {
                    if ready_to_publish_rows.is_empty() {
                        log::debug!("{}: no rows ready to fire for dealine {}", method_name, deadline);
                        break;
                    } else {
                        let publish_futures = ready_to_publish_rows.into_iter().map(|row| self.prepare_to_publish(row));

                        let results = futures::future::join_all(publish_futures).await;

                        // closure to gather ids from results vector and ignore error from result

                        let ids: Vec<String> = results.into_iter().filter_map(|result| result.ok()).collect();
                        if !ids.is_empty() {
                            let _ = self.delete_fired_records_from_db(&ids).await;
                            // event!(
                            //     tracing::Level::INFO,
                            //     "number of rows published successfully and deleted from DB: {:?}",
                            //     ids.len()
                            // );
                            log::debug!("{}: number of rows published successfully and deleted from DB {}", method_name, ids.len());
                            break;
                        }
                    }
                }
                Err(e) => {
                    // if DB is locked, log a warning and continue
                    if e.contains("could not serialize access due to concurrent update") {
                        log::warn!("{}: could not serialize access due to concurrent update", method_name);
                    }

                    log::error!("{}: occurred while processing message ready {}", method_name, e);
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
            self.processor_message_ready(node_id).await;

            delay_controller.sleep().await;
        }
    }
}
