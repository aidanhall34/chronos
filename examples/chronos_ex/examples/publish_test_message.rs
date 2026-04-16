/// Publishes a single test message to the Chronos input Kafka topic.
///
/// Required environment variables (same as the main Chronos service):
///   KAFKA_HOST, KAFKA_PORT, KAFKA_CLIENT_ID, KAFKA_GROUP_ID,
///   KAFKA_IN_TOPIC, KAFKA_OUT_TOPIC, KAFKA_USERNAME, KAFKA_PASSWORD
///
/// Optional environment variables:
///   CHRONOS_DEADLINE  RFC3339 timestamp for the message deadline.
///                     Defaults to 1 minute in the past, which causes
///                     Chronos to fire the message immediately and generate
///                     observable msg_jitter metrics.
///   CHRONOS_MSG_ID    Override the generated message UUID.
use chrono::{Duration, Utc};
use chronos_bin::kafka::config::KafkaConfig;
use chronos_bin::kafka::producer::KafkaProducer;
use std::collections::HashMap;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenv::dotenv().ok();

    let msg_id = std::env::var("CHRONOS_MSG_ID").unwrap_or_else(|_| Uuid::new_v4().to_string());

    // Default: 1 minute in the past so Chronos fires immediately (exercises jitter metrics).
    // Override with a future timestamp to test the "store and delay" path.
    let deadline = std::env::var("CHRONOS_DEADLINE").unwrap_or_else(|_| (Utc::now() - Duration::minutes(1)).to_rfc3339());

    let payload = serde_json::json!({
        "source": "integration-test",
        "message_id": msg_id,
        "sent_at": Utc::now().to_rfc3339(),
    })
    .to_string();

    let mut headers = HashMap::new();
    headers.insert("chronosMessageId".to_string(), msg_id.clone());
    headers.insert("chronosDeadline".to_string(), deadline.clone());

    println!("Publishing test message");
    println!("  id:       {}", msg_id);
    println!("  deadline: {}", deadline);
    println!("  payload:  {}", payload);

    let kafka_config = KafkaConfig::from_env();
    let producer = KafkaProducer::new(&kafka_config);

    match producer.kafka_publish(payload, Some(headers), msg_id.clone()).await {
        Ok(id) => println!("✓ Published successfully (returned id: {})", id),
        Err(e) => {
            eprintln!("✗ Failed to publish: {}", e);
            std::process::exit(1);
        }
    }
}
