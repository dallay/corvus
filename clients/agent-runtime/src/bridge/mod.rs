use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BridgeProtocolVersion {
    V1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BridgeTransportKind {
    Sse,
    Websocket,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteBridgeRequest {
    pub protocol_version: BridgeProtocolVersion,
    pub transport: BridgeTransportKind,
    pub session_scope: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RemoteBridgeAvailability {
    Deferred,
    Rejected { reason: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BridgeEnvelope {
    pub version: BridgeProtocolVersion,
    pub session_scope: String,
    pub sequence: u64,
    pub transport: BridgeTransportKind,
    pub kind: String,
    pub payload: serde_json::Value,
}

impl BridgeEnvelope {
    pub fn to_sse_frame(&self) -> anyhow::Result<String> {
        Ok(format!(
            "event: {}\nid: {}\ndata: {}\n\n",
            self.kind,
            self.sequence,
            serde_json::to_string(self)?
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_envelope_serializes_metadata_only_seam() {
        let envelope = BridgeEnvelope {
            version: BridgeProtocolVersion::V1,
            session_scope: "sess-1".to_string(),
            sequence: 7,
            transport: BridgeTransportKind::Sse,
            kind: "validation_failed".to_string(),
            payload: serde_json::json!({"reason": "deferred"}),
        };
        let frame = envelope.to_sse_frame().unwrap();
        assert!(frame.contains("event: validation_failed"));
        assert!(frame.contains("\"session_scope\":\"sess-1\""));
    }

    #[test]
    fn remote_bridge_request_and_availability_remain_metadata_only() {
        let request = RemoteBridgeRequest {
            protocol_version: BridgeProtocolVersion::V1,
            transport: BridgeTransportKind::Websocket,
            session_scope: "scope-1".to_string(),
        };
        let availability = RemoteBridgeAvailability::Rejected {
            reason: "remote bridge transport is deferred in this slice".to_string(),
        };

        let request_json = serde_json::to_value(&request).unwrap();
        let availability_json = serde_json::to_value(&availability).unwrap();

        assert_eq!(request_json["transport"], "websocket");
        assert_eq!(availability_json["kind"], "rejected");
        assert!(availability_json["reason"]
            .as_str()
            .unwrap_or("")
            .contains("deferred"));
    }
}
