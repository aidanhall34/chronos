use crate::postgres::pg::Pg;
use crate::utils::config::ChronosConfig;
use chrono::Utc;
use std::sync::Arc;
use std::time::Duration;
use tokio_postgres::Row;

#[derive(Debug)]
pub struct FailureDetector {
    pub(crate) data_store: Arc<Pg>,
}

impl FailureDetector {
    pub async fn run(&self) {
        log::info!("Monitoring On!");
        loop {
            let _ = tokio::time::sleep(Duration::from_secs(ChronosConfig::from_env().monitor_db_poll)).await;

            let _ = &self.monitor_failed_fire_records().await;
        }
    }

    #[tracing::instrument(skip_all, fields(error))]
    async fn reset_to_init_db(&self, fetched_rows: &std::vec::Vec<tokio_postgres::Row>) {
        if !fetched_rows.is_empty() {
            if let Err(e) = &self.data_store.reset_to_init_db(fetched_rows).await {
                tracing::Span::current().record("error", e);
                log::error!("error in monitor reset_to_init {}", e);
            } else {
                log::debug!("reset_to_init_db success for {:?}", fetched_rows)
            }
        }
    }

    async fn retry_reset_to_init_db_loop(&self, fetched_rows: &Vec<Row>) {
        //retry loop
        let method_name = "retry_loop";
        let max_retry_count = 3;
        let mut retry_count = 0;
        while let Err(outcome_error) = &self.data_store.reset_to_init_db(&fetched_rows).await {
            log::error!("{}: error occurred {}", method_name, outcome_error);
            retry_count += 1;
            if retry_count == max_retry_count {
                log::error!(
                    "{}: max retry count {} reached by node {:?} for resetting to init db ",
                    method_name,
                    max_retry_count,
                    fetched_rows
                );
                break;
            }
        }
    }

    #[tracing::instrument(skip_all, fields(error, fail_to_fire_rows))]
    async fn monitor_failed_fire_records(&self) {
        match &self
            .data_store
            .failed_to_fire_db(&(Utc::now() - Duration::from_secs(ChronosConfig::from_env().fail_detect_interval)))
            .await
        {
            Ok(fetched_rows) => {
                tracing::Span::current().record("fail_to_fire_rows", fetched_rows.len());
                if !fetched_rows.is_empty() {
                    self.retry_reset_to_init_db_loop(fetched_rows).await;
                }
            }
            Err(e) => {
                log::error!("error in monitor {}", e);
                tracing::Span::current().record("error", e.to_string());
            }
        }
    }
}
