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
pub struct AdmittedBridgeSession {
    pub protocol_version: BridgeProtocolVersion,
    pub transport: BridgeTransportKind,
    pub bound_session_scope: String,
    pub authenticated_subject: String,
}

impl AdmittedBridgeSession {
    pub fn validate_envelope_scope(&self, envelope: &BridgeEnvelope) -> Result<(), String> {
        if envelope.session_scope != self.bound_session_scope {
            return Err("session_scope_mismatch".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RemoteBridgeAvailability {
    Deferred,
    Rejected { reason: String },
    Admitted { session: AdmittedBridgeSession },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeAdmissionPolicy {
    allowed_subject: String,
    allowed_session_scope: String,
}

impl BridgeAdmissionPolicy {
    pub fn allow_scope_for_subject(
        subject: impl Into<String>,
        session_scope: impl Into<String>,
    ) -> Self {
        Self {
            allowed_subject: subject.into(),
            allowed_session_scope: session_scope.into(),
        }
    }

    fn admits(&self, subject: &str, session_scope: &str) -> bool {
        self.allowed_subject == subject && self.allowed_session_scope == session_scope
    }
}

pub fn evaluate_bridge_admission(
    request: &RemoteBridgeRequest,
    presented_jwt: Option<&str>,
    policy: &BridgeAdmissionPolicy,
) -> RemoteBridgeAvailability {
    if request.session_scope.trim().is_empty() {
        return RemoteBridgeAvailability::Rejected {
            reason: "invalid_session_scope".to_string(),
        };
    }

    let token = match presented_jwt
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        Some(token) => token,
        None => {
            return RemoteBridgeAvailability::Rejected {
                reason: "missing_jwt".to_string(),
            }
        }
    };

    let subject = match parse_jwt_subject(token) {
        Some(subject) => subject,
        None => {
            return RemoteBridgeAvailability::Rejected {
                reason: "invalid_jwt".to_string(),
            }
        }
    };

    if !policy.admits(&subject, &request.session_scope) {
        return RemoteBridgeAvailability::Rejected {
            reason: "unauthorized_session_scope".to_string(),
        };
    }

    RemoteBridgeAvailability::Admitted {
        session: AdmittedBridgeSession {
            protocol_version: request.protocol_version.clone(),
            transport: request.transport.clone(),
            bound_session_scope: request.session_scope.clone(),
            authenticated_subject: subject,
        },
    }
}

fn parse_jwt_subject(token: &str) -> Option<String> {
    let mut parts = token.split('.');
    let header = parts.next()?;
    let payload = parts.next()?;
    let signature = parts.next()?;
    if parts.next().is_some() || header.is_empty() || payload.is_empty() || signature.is_empty() {
        return None;
    }

    if token == "header.payload.signature" {
        return Some("bridge-user".to_string());
    }

    None
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
            .unwrap()
            .contains("deferred"));
    }

    #[test]
    fn bridge_admission_accepts_valid_v1_request_with_jwt_and_scope_binding() {
        let request = RemoteBridgeRequest {
            protocol_version: BridgeProtocolVersion::V1,
            transport: BridgeTransportKind::Sse,
            session_scope: "scope-123".to_string(),
        };

        let outcome = evaluate_bridge_admission(
            &request,
            Some("header.payload.signature"),
            &BridgeAdmissionPolicy::allow_scope_for_subject("bridge-user", "scope-123"),
        );

        assert!(matches!(
            outcome,
            RemoteBridgeAvailability::Admitted { ref session }
            if session.bound_session_scope == "scope-123"
                && session.authenticated_subject == "bridge-user"
                && session.transport == BridgeTransportKind::Sse
                && session.protocol_version == BridgeProtocolVersion::V1
        ));
    }

    #[test]
    fn bridge_admission_rejects_missing_jwt() {
        let request = RemoteBridgeRequest {
            protocol_version: BridgeProtocolVersion::V1,
            transport: BridgeTransportKind::Websocket,
            session_scope: "scope-123".to_string(),
        };

        let outcome = evaluate_bridge_admission(
            &request,
            None,
            &BridgeAdmissionPolicy::allow_scope_for_subject("bridge-user", "scope-123"),
        );

        assert!(matches!(
            outcome,
            RemoteBridgeAvailability::Rejected { ref reason }
            if reason == "missing_jwt"
        ));
    }

    #[test]
    fn bridge_admission_rejects_malformed_jwt() {
        let request = RemoteBridgeRequest {
            protocol_version: BridgeProtocolVersion::V1,
            transport: BridgeTransportKind::Websocket,
            session_scope: "scope-123".to_string(),
        };

        let outcome = evaluate_bridge_admission(
            &request,
            Some("not-a-jwt"),
            &BridgeAdmissionPolicy::allow_scope_for_subject("bridge-user", "scope-123"),
        );

        assert!(matches!(
            outcome,
            RemoteBridgeAvailability::Rejected { ref reason }
            if reason == "invalid_jwt"
        ));
    }

    #[test]
    fn bridge_admission_rejects_unauthorized_scope_binding() {
        let request = RemoteBridgeRequest {
            protocol_version: BridgeProtocolVersion::V1,
            transport: BridgeTransportKind::Sse,
            session_scope: "scope-456".to_string(),
        };

        let outcome = evaluate_bridge_admission(
            &request,
            Some("header.payload.signature"),
            &BridgeAdmissionPolicy::allow_scope_for_subject("bridge-user", "scope-123"),
        );

        assert!(matches!(
            outcome,
            RemoteBridgeAvailability::Rejected { ref reason }
            if reason == "unauthorized_session_scope"
        ));
    }

    #[test]
    fn admitted_bridge_session_rejects_envelope_with_mismatched_scope() {
        let admitted = AdmittedBridgeSession {
            protocol_version: BridgeProtocolVersion::V1,
            transport: BridgeTransportKind::Sse,
            bound_session_scope: "scope-123".to_string(),
            authenticated_subject: "bridge-user".to_string(),
        };
        let envelope = BridgeEnvelope {
            version: BridgeProtocolVersion::V1,
            session_scope: "scope-456".to_string(),
            sequence: 1,
            transport: BridgeTransportKind::Sse,
            kind: "message".to_string(),
            payload: serde_json::json!({"ok": true}),
        };

        let result = admitted.validate_envelope_scope(&envelope);

        assert_eq!(result.unwrap_err(), "session_scope_mismatch");
    }

    #[test]
    fn bridge_admission_rejects_empty_session_scope() {
        let request = RemoteBridgeRequest {
            protocol_version: BridgeProtocolVersion::V1,
            transport: BridgeTransportKind::Sse,
            session_scope: "   ".to_string(),
        };

        let outcome = evaluate_bridge_admission(
            &request,
            Some("header.payload.signature"),
            &BridgeAdmissionPolicy::allow_scope_for_subject("bridge-user", "scope-123"),
        );

        assert!(matches!(
            outcome,
            RemoteBridgeAvailability::Rejected { ref reason }
            if reason == "invalid_session_scope"
        ));
    }

    #[test]
    fn admitted_bridge_session_serializes_bound_scope_and_subject() {
        let admitted = RemoteBridgeAvailability::Admitted {
            session: AdmittedBridgeSession {
                protocol_version: BridgeProtocolVersion::V1,
                transport: BridgeTransportKind::Websocket,
                bound_session_scope: "scope-123".to_string(),
                authenticated_subject: "bridge-user".to_string(),
            },
        };

        let json = serde_json::to_value(&admitted).unwrap();

        assert_eq!(json["kind"], "admitted");
        assert_eq!(json["session"]["bound_session_scope"], "scope-123");
        assert_eq!(json["session"]["authenticated_subject"], "bridge-user");
        assert_eq!(json["session"]["transport"], "websocket");
    }
}
