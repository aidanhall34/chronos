use chrono::{DateTime, Utc};
use deadpool_postgres::{Config, GenericClient, ManagerConfig, Object, Pool, PoolConfig, Runtime, Transaction};
use log::error;
use std::time::{Duration, Instant};
use tokio_postgres::error::SqlState;
use tokio_postgres::types::ToSql;
use tokio_postgres::{NoTls, Row};
use uuid::Uuid;

use crate::postgres::config::PgConfig;
use crate::postgres::errors::PgError;

use tracing::event;

#[derive(Clone, Debug)]
pub struct Pg {
    pub pool: Pool,
}

#[derive(Debug)]
pub struct TableInsertColumns<'a> {
    pub id: &'a str,
    pub deadline: DateTime<Utc>,
    pub message_headers: serde_json::Value,
    pub message_key: &'a str,
    pub message_value: serde_json::Value,
}

#[derive(Debug)]
pub struct TableRow<'a> {
    pub id: &'a str,
    pub deadline: DateTime<Utc>,
    pub readied_at: DateTime<Utc>,
    pub readied_by: Uuid,
    pub message_headers: serde_json::Value,
    pub message_key: &'a str,
    pub message_value: serde_json::Value,
}

#[derive(Debug)]
pub struct TableInsertRow<'a> {
    pub id: &'a str,
    pub deadline: DateTime<Utc>,
    pub message_headers: &'a serde_json::Value,
    pub message_key: &'a str,
    pub message_value: &'a serde_json::Value,
}
#[derive(Debug)]
pub struct GetReady {
    pub readied_at: DateTime<Utc>,
    pub readied_by: Uuid,
    pub deadline: DateTime<Utc>,
    // pub limit: i64,
    // pub order: &'a str,
}

struct PgTxn<'a> {
    txn: Transaction<'a>,
}

struct PgAccess {
    client: Object,
}

impl PgAccess {
    pub async fn build_txn(&mut self) -> Result<PgTxn<'_>, PgError> {
        let txn = self
            .client
            .build_transaction()
            .isolation_level(tokio_postgres::IsolationLevel::RepeatableRead)
            .start();

        match txn.await {
            Ok(txn) => Ok(PgTxn { txn }),
            Err(e) => {
                error!("Unable to start transaction: {}", e);
                Err(PgError::UnknownException(e))
            }
        }
    }
}

impl Pg {
    pub async fn new(pg_config: PgConfig) -> Result<Self, PgError> {
        let port = pg_config.port.parse::<u16>().unwrap(); // make the connection fail
        let mut config = Config::new();
        config.dbname = Some(pg_config.database);
        config.user = Some(pg_config.user);
        config.password = Some(pg_config.password);
        config.host = Some(pg_config.host);
        config.port = Some(port);
        config.manager = Some(ManagerConfig {
            recycling_method: deadpool_postgres::RecyclingMethod::Fast,
        });
        config.pool = Some(PoolConfig::new(pg_config.pool_size));

        let pool = config.create_pool(Some(Runtime::Tokio1), NoTls).map_err(PgError::CreatePool)?;

        {
            //test connection
            let mut tmp_list: Vec<Object> = Vec::new();
            for _ in 1..=pg_config.pool_size {
                let client = match pool.get().await {
                    Ok(client) => client,
                    Err(e) => {
                        error!("error::: Cannot get client from the pool while setting transaction isolation level {:?}", &e);
                        event!(
                            tracing::Level::ERROR,
                            error = %e,
                            "error::: Cannot get client from the pool while setting transaction isolation",
                        );
                        return Err(PgError::GetClientFromPool(e));
                    }
                };
                let _ = client
                    .execute("SET SESSION CHARACTERISTICS AS TRANSACTION ISOLATION LEVEL REPEATABLE READ;", &[])
                    .await
                    .is_ok();
                tmp_list.push(client);
            }
        }

        for _ in 1..=pg_config.pool_size {
            let client = match pool.get().await {
                Ok(client) => client,
                Err(e) => {
                    error!("error::: Cannot get client from the pool {:?}", e);
                    return Err(PgError::GetClientFromPool(e));
                }
            };

            let rs = client.query_one("show transaction_isolation", &[]).await?;
            let value: String = rs.get(0);
            log::debug!("init: db-isolation-level: {}", value);
        }

        log::info!("pool.status: {:?}", pool.status());
        Ok(Pg { pool })
    }

    pub async fn get_client(&self) -> Result<Object, PgError> {
        match self.pool.get().await {
            Err(e) => {
                error!("error::: {:?}", e);
                event!(tracing::Level::ERROR,error=%e, "pg client creation error");
                Err(PgError::GetClientFromPool(e))
            }
            Ok(client) => Ok(client),
        }
    }
}

impl Pg {
    #[tracing::instrument(skip_all)]
    pub(crate) async fn insert_to_delay_db(&self, params: &TableInsertRow<'_>) -> Result<u64, PgError> {
        let pg_client = self.get_client().await?;
        let mut pg_access = PgAccess { client: pg_client };
        let pg_txn: PgTxn = pg_access.build_txn().await?;
        let insert_query = "INSERT INTO hanger (id, deadline,  message_headers, message_key, message_value)
        VALUES ($1, $2 ,$3, $4, $5 )";

        let query_execute_instant = Instant::now();
        let stmt = pg_txn.txn.prepare(insert_query).await?;
        let outcome = pg_txn
            .txn
            .execute(
                &stmt,
                &[
                    &params.id,
                    &params.deadline,
                    &params.message_headers,
                    &params.message_key,
                    &params.message_value,
                ],
            )
            .await?;
        let time_elapsed = query_execute_instant.elapsed();
        if time_elapsed > Duration::from_millis(100) {
            log::warn!("insert_to_delay query_execute_instant: {:?} ", time_elapsed);
        }

        if outcome > 0 {
            event!(tracing::Level::INFO, "insert_to_delay success");
            let cmt_rdy = pg_txn.txn.commit().await;
            if let Err(e) = cmt_rdy {
                error!("Unable to commit: {}. The original transaction updated rows", e);
                return Err(PgError::UnknownException(e));
            }
        }
        Ok(outcome)
    }

    #[tracing::instrument(skip_all)]
    pub(crate) async fn delete_fired(&self, ids: &[String]) -> Result<u64, String> {
        // let query_execute_instant = Instant::now();
        let pg_client = match self.get_client().await {
            Ok(client) => client,
            Err(e) => {
                error!("delete_fired: Unable to get client: {}", e);
                return Err(format!("delete_fired: Unable to get client: {}", e));
            }
        };
        let mut pg_access = PgAccess { client: pg_client };
        let pg_txn: PgTxn = match pg_access.build_txn().await {
            Ok(txn) => txn,
            Err(e) => {
                error!("delete_fired: Unable to start transaction: {}", e);
                return Err(format!("delete_fired: Unable to start transaction: {}", e));
            }
        };

        let values_as_slice: Vec<_> = ids.iter().map(|x| x as &(dyn ToSql + Sync)).collect();

        let mut query: String = "DELETE FROM hanger WHERE id IN (".to_owned();
        for i in 0..ids.len() {
            query = query + "$" + (i + 1).to_string().as_str() + ",";
        }
        query = match query.strip_suffix(',') {
            Some(query) => query.to_string(),
            None => {
                error!("reset_to_init: Unable to strip suffix");
                return Err("reset_to_init: Unable to strip suffix".to_string());
            }
        };
        query += ")";

        let stmt = match pg_txn.txn.prepare(query.as_str()).await {
            Ok(stmt) => stmt,
            Err(e) => {
                error!("delete_fired: Unable to prepare query: {}", e);
                return Err(format!("delete_fired: Unable to prepare query: {}", e));
            }
        };
        let response = pg_txn.txn.execute(&stmt, &values_as_slice).await;
        match response {
            Ok(resp) => {
                let cmt_rdy = pg_txn.txn.commit().await;
                if let Err(e) = cmt_rdy {
                    error!("delete_fired: Unable to commit: {}. The original transaction updated: rows", e,);
                    return Err(format!("Unable to commit: {}. The original transaction updated rows", e));
                }
                Ok(resp)
            }
            Err(e) => {
                if let Some(err_code) = e.code() {
                    if err_code == &SqlState::T_R_SERIALIZATION_FAILURE {
                        error!("delete_fired: Unable to execute txn due to : {}", e);
                        return Err(format!("delete_fired: Unable to execute txn due to : {}", e));
                    }
                }
                Err(format!("delete_fired: Unknow exception {:?}", e))
            }
        }
    }

    #[tracing::instrument(skip_all)]
    pub(crate) async fn ready_to_fire_db(&self, param: &GetReady) -> Result<Vec<Row>, String> {
        //TODO handle get client error gracefully
        let pg_client = match self.get_client().await {
            Ok(client) => client,
            Err(e) => {
                error!("ready_to_fire: Unable to get client: {}", e);
                return Err(format!("ready_to_fire: Unable to get client: {}", e));
            }
        };
        let mut pg_access = PgAccess { client: pg_client };
        let pg_txn: PgTxn = match pg_access.build_txn().await {
            Ok(txn) => txn,
            Err(e) => {
                error!("delete_fired: Unable to start transaction: {}", e);
                return Err(format!("delete_fired: Unable to start transaction: {}", e));
            }
        };

        let ready_query = "UPDATE hanger SET readied_at = $1, readied_by = $2 where deadline < $3 AND readied_at IS NULL RETURNING id, deadline, readied_at, readied_by, message_headers, message_key, message_value";

        // println!("ready_query: {}", ready_query);
        // if let Ok(stmt) = pg_txn.txn.prepare(ready_query).await {
        let stmt = match pg_txn.txn.prepare(ready_query).await {
            Ok(stmt) => stmt,
            Err(e) => {
                error!("ready_to_fire: Unable to prepare query: {}", e);
                return Err(format!("ready_to_fire: Unable to prepare query: {}", e));
            }
        };

        let query_execute_instant = Instant::now();
        let response = pg_txn.txn.query(&stmt, &[&param.readied_at, &param.readied_by, &param.deadline]).await;

        match response {
            Ok(resp) => {
                let cmt_rdy = pg_txn.txn.commit().await;
                if let Err(e) = cmt_rdy {
                    error!("Unable to commit: {}. The original transaction updated: {:?} rows", e, resp);
                    return Err(format!(
                        "ready_to_fire: Unable to commit: {}. The original transaction updated: {:?} rows",
                        e, resp
                    ));
                }
                let time_elapsed = query_execute_instant.elapsed();
                if time_elapsed > Duration::from_millis(100) {
                    log::warn!(" ready_to_fire query_execute_instant: {:?} params: {:?}", time_elapsed, param);
                }
                Ok(resp)
            }
            Err(e) => {
                if let Some(err_code) = e.code() {
                    if err_code == &SqlState::T_R_SERIALIZATION_FAILURE {
                        error!("ready_to_fire: Unable to execute txn due to : {}", e);
                        return Err(format!("ready_to_fire: Unable to execute txn due to : {}", e));
                    }
                }
                error!("ready_to_fire: Unknow exception {:?}", e);
                Err(format!("ready_to_fire: Unknow exception {:?}", e))
            }
        }
    }

    #[tracing::instrument(skip_all)]
    pub(crate) async fn failed_to_fire_db(&self, delay_time: &DateTime<Utc>) -> Result<Vec<Row>, PgError> {
        let query_execute_instant = Instant::now();
        let pg_client = self.get_client().await?;
        let mut pg_access = PgAccess { client: pg_client };
        let pg_txn: PgTxn = pg_access.build_txn().await?;

        log::debug!("failed_to_fire param delay_time: {:?}", delay_time);
        let get_query = "SELECT * from hanger where readied_at > $1 ORDER BY deadline DESC";
        let stmt = pg_txn.txn.prepare(get_query).await?;

        let response = pg_txn.txn.query(&stmt, &[&delay_time]).await?;
        let time_elapsed = query_execute_instant.elapsed();
        if time_elapsed > Duration::from_millis(100) {
            log::warn!(" failed_to_fire query_execute_instant: {:?} ", time_elapsed);
        }

        Ok(response)
    }

    #[tracing::instrument(skip_all)]
    pub(crate) async fn reset_to_init_db(&self, to_init_list: &Vec<Row>) -> Result<Vec<String>, String> {
        let query_execute_instant = Instant::now();
        let mut id_list = Vec::<String>::new();
        for row in to_init_list {
            id_list.push(row.get("id"));
        }

        let pg_client = match self.get_client().await {
            Ok(client) => client,
            Err(e) => {
                error!("reset_to_init: Unable to get client: {}", e);
                return Err(format!("reset_to_init: Unable to get client: {}", e));
            }
        };
        let mut pg_access = PgAccess { client: pg_client };
        let pg_txn: PgTxn = match pg_access.build_txn().await {
            Ok(txn) => txn,
            Err(e) => {
                error!("delete_fired: Unable to start transaction: {}", e);
                return Err(format!("delete_fired: Unable to start transaction: {}", e));
            }
        };

        let values_as_slice: Vec<_> = id_list.iter().map(|x| x as &(dyn ToSql + Sync)).collect();

        let mut query: String = "UPDATE hanger SET readied_at=null , readied_by=null  WHERE id IN  (".to_owned();
        for i in 0..id_list.len() {
            query = query + "$" + (i + 1).to_string().as_str() + ",";
        }
        query = match query.strip_suffix(',') {
            Some(query) => query.to_string(),
            None => {
                error!("reset_to_init: Unable to strip suffix");
                return Err("reset_to_init: Unable to strip suffix".to_string());
            }
        };
        query += ")";

        println!("query: {}", query);

        let stmt = match pg_txn.txn.prepare(query.as_str()).await {
            Ok(stmt) => stmt,
            Err(e) => {
                error!("reset_to_init: Unable to prepare query: {}", e);
                return Err(format!("reset_to_init: Unable to prepare query: {}", e));
            }
        };
        let response = pg_txn.txn.execute(&stmt, &values_as_slice[..]).await;

        match response {
            Ok(resp) => {
                let cmt_rdy = pg_txn.txn.commit().await;
                if let Err(e) = cmt_rdy {
                    error!("Unable to commit: {}. The original transaction updated: {:?} rows", e, resp);
                    return Err(format!("Unable to commit: {}. The original transaction updated: {:?} rows", e, resp));
                }
                let time_elapsed = query_execute_instant.elapsed();
                if time_elapsed > Duration::from_millis(100) {
                    log::warn!(" ready_to_fire query_execute_instant: {:?} ", time_elapsed);
                }
                Ok(id_list)
            }
            Err(e) => {
                error!("reset_to_init: Unable to execute txn due to : {}", e);
                if let Some(err_code) = e.code() {
                    if err_code == &SqlState::T_R_SERIALIZATION_FAILURE {
                        error!("reset_to_init: Unable to execute txn due to : {}", e);
                        return Err(format!("reset_to_init: Unable to execute txn due to : {}", e));
                    }
                }
                Err(format!("reset_to_init: Unknow exception {:?}", e))
            }
        }
    }
}
