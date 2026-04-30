use rdkafka::message::{BorrowedHeaders, BorrowedMessage, Header, Headers, OwnedHeaders};
use rdkafka::Message;
use std::collections::HashMap;

pub static CHRONOS_ID: &str = "chronosMessageId";
pub static DEADLINE: &str = "chronosDeadline";

pub fn required_headers(message: &BorrowedMessage) -> Option<HashMap<String, String>> {
    if let Some(headers) = message.headers() {
        if headers_check(headers) {
            let reqd_headers = headers.iter().fold(HashMap::<String, String>::new(), |mut acc, header| {
                if let Some(value) = header.value {
                    let value: String = String::from_utf8_lossy(value).into_owned();
                    acc.insert(header.key.to_string(), value);
                    acc
                } else {
                    acc
                }
            });
            return Some(reqd_headers);
        }
    }
    None
}

pub fn into_headers(headers: &HashMap<String, String>) -> OwnedHeaders {
    headers.iter().fold(OwnedHeaders::new(), |acc, header| {
        let (key, value) = header;
        let o_header = Header { key, value: Some(value) };
        acc.insert(o_header)
    })
}

pub fn headers_check(headers: &BorrowedHeaders) -> bool {
    // println!("headers_check {:?}", headers);
    let outcome = headers
        .iter()
        .filter(|h| {
            let header_keys = [CHRONOS_ID, DEADLINE];
            header_keys.contains(&h.key) && h.value.is_some()
        })
        .count();

    outcome == 2
}

pub fn get_payload_utf8<'a>(message: &'a BorrowedMessage<'_>) -> Option<&'a [u8]> {
    message.payload()
}

pub fn get_message_key(message: &BorrowedMessage) -> Option<String> {
    message.key().map(|key| String::from_utf8_lossy(key).to_string())
    // let key = String::from_utf8_lossy(.expect("No key found for message")).to_string();
    // key
}
