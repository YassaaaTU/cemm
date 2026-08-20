use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const SERVICE_PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServiceRequest {
    pub id: u64,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServiceMessage {
    Ready {
        protocol_version: u32,
    },
    Response {
        id: u64,
        result: Value,
    },
    Error {
        id: u64,
        error: String,
    },
    Event {
        id: u64,
        name: String,
        payload: Value,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServiceEvent {
    pub request_id: u64,
    pub name: String,
    pub payload: Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_round_trips_as_one_json_value() {
        let request = ServiceRequest {
            id: 42,
            method: "ping".to_string(),
            params: serde_json::json!({ "probe": true }),
        };

        let encoded = serde_json::to_string(&request).expect("request should serialize");
        assert!(!encoded.contains('\n'));
        assert_eq!(
            serde_json::from_str::<ServiceRequest>(&encoded).expect("request should deserialize"),
            request
        );
    }

    #[test]
    fn tagged_messages_keep_response_and_event_shapes_distinct() {
        let response = ServiceMessage::Response {
            id: 7,
            result: serde_json::json!({ "ok": true }),
        };
        let event = ServiceMessage::Event {
            id: 7,
            name: "install-progress".to_string(),
            payload: serde_json::json!({ "progress": 25 }),
        };

        let response_json = serde_json::to_value(response).expect("response should serialize");
        let event_json = serde_json::to_value(event).expect("event should serialize");

        assert_eq!(response_json["type"], "response");
        assert_eq!(event_json["type"], "event");
    }
}
