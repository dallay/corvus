use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Stop {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatCompletionMessage>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub n: Option<u32>,
    pub stop: Option<Stop>,
    pub max_tokens: Option<u32>,
    pub presence_penalty: Option<f64>,
    pub frequency_penalty: Option<f64>,
    pub user: Option<String>,
    #[serde(default)]
    pub stream: Option<bool>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionMessage {
    pub role: String,
    pub content: Option<serde_json::Value>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatCompletionChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChoice {
    pub index: u32,
    pub message: ChatCompletionMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelObject {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub owned_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelListResponse {
    pub object: String,
    pub data: Vec<ModelObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayErrorResponse {
    pub error: GatewayErrorBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayErrorBody {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: String,
    pub code: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn chat_completion_request_deserializes_minimal_payload() {
        let value = json!({
            "model": "gpt-4o",
            "messages": [{ "role": "user", "content": "Hello" }]
        });

        let request: ChatCompletionRequest = serde_json::from_value(value).unwrap();

        assert_eq!(request.model, "gpt-4o");
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.messages[0].role, "user");
        assert_eq!(request.messages[0].content, Some(json!("Hello")));
        assert_eq!(request.stream, None);
    }

    #[test]
    fn chat_completion_request_deserializes_stream_variants() {
        let omitted: ChatCompletionRequest = serde_json::from_value(json!({
            "model": "gpt-4o",
            "messages": [{ "role": "user", "content": "Hello" }]
        }))
        .unwrap();
        assert_eq!(omitted.stream, None);

        let stream_true: ChatCompletionRequest = serde_json::from_value(json!({
            "model": "gpt-4o",
            "messages": [{ "role": "user", "content": "Hello" }],
            "stream": true
        }))
        .unwrap();
        assert_eq!(stream_true.stream, Some(true));

        let stream_false: ChatCompletionRequest = serde_json::from_value(json!({
            "model": "gpt-4o",
            "messages": [{ "role": "user", "content": "Hello" }],
            "stream": false
        }))
        .unwrap();
        assert_eq!(stream_false.stream, Some(false));
    }

    #[test]
    fn chat_completion_request_deserializes_typed_optional_fields() {
        let request: ChatCompletionRequest = serde_json::from_value(json!({
            "model": "gpt-4o",
            "messages": [{ "role": "user", "content": "Hello" }],
            "temperature": 0.7,
            "top_p": 0.8,
            "n": 2,
            "stop": "\n",
            "max_tokens": 128,
            "presence_penalty": 0.1,
            "frequency_penalty": 0.2,
            "user": "user-123",
            "stream": false
        }))
        .unwrap();

        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.top_p, Some(0.8));
        assert_eq!(request.n, Some(2));
        assert_eq!(request.stop, Some(Stop::Single("\n".to_string())));
        assert_eq!(request.max_tokens, Some(128));
        assert_eq!(request.presence_penalty, Some(0.1));
        assert_eq!(request.frequency_penalty, Some(0.2));
        assert_eq!(request.user, Some("user-123".to_string()));
    }

    #[test]
    fn chat_completion_request_deserializes_stop_array() {
        let request: ChatCompletionRequest = serde_json::from_value(json!({
            "model": "gpt-4o",
            "messages": [{ "role": "user", "content": "Hello" }],
            "stop": ["\n", "END"]
        }))
        .unwrap();

        assert_eq!(
            request.stop,
            Some(Stop::Multiple(vec!["\n".to_string(), "END".to_string()]))
        );
    }

    #[test]
    fn chat_completion_request_tolerates_and_preserves_unknown_fields() {
        let value = json!({
            "model": "gpt-4o",
            "messages": [{
                "role": "assistant",
                "content": null,
                "tool_call_id": "call_123"
            }],
            "logprobs": true
        });

        let request: ChatCompletionRequest = serde_json::from_value(value.clone()).unwrap();
        let round_trip = serde_json::to_value(&request).unwrap();

        assert_eq!(round_trip["logprobs"], json!(true));
        assert_eq!(round_trip["messages"][0]["tool_call_id"], json!("call_123"));
    }

    #[test]
    fn chat_completion_response_round_trips_with_usage() {
        let response = ChatCompletionResponse {
            id: "chatcmpl-abc".to_string(),
            object: "chat.completion".to_string(),
            created: 1_700_000_000,
            model: "gpt-4o".to_string(),
            choices: vec![ChatCompletionChoice {
                index: 0,
                message: ChatCompletionMessage {
                    role: "assistant".to_string(),
                    content: Some(json!("Hello")),
                    extra: Default::default(),
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(Usage {
                prompt_tokens: 10,
                completion_tokens: 20,
                total_tokens: 30,
            }),
            system_fingerprint: Some("fp_123".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        let decoded: ChatCompletionResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.id, response.id);
        assert_eq!(decoded.object, response.object);
        assert_eq!(decoded.created, response.created);
        assert_eq!(decoded.model, response.model);
        assert_eq!(decoded.choices.len(), 1);
        assert_eq!(decoded.usage.unwrap().total_tokens, 30);
        assert_eq!(decoded.system_fingerprint, Some("fp_123".to_string()));
    }

    #[test]
    fn model_object_serializes_to_expected_shape() {
        let model = ModelObject {
            id: "gpt-4o".to_string(),
            object: "model".to_string(),
            created: 1_700_000_000,
            owned_by: "rook".to_string(),
        };

        let json = serde_json::to_value(&model).unwrap();

        assert_eq!(
            json,
            json!({
                "id": "gpt-4o",
                "object": "model",
                "created": 1_700_000_000,
                "owned_by": "rook"
            })
        );
    }

    #[test]
    fn model_list_response_serializes_empty_data() {
        let response = ModelListResponse {
            object: "list".to_string(),
            data: vec![],
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json, json!({ "object": "list", "data": [] }));
    }

    #[test]
    fn gateway_error_response_serializes_openai_shape() {
        let response = GatewayErrorResponse {
            error: GatewayErrorBody {
                message: "no route configured for model 'unknown-model'".to_string(),
                error_type: "server_error".to_string(),
                code: Some("model_not_found".to_string()),
            },
        };

        let json = serde_json::to_value(&response).unwrap();

        assert_eq!(json["error"]["message"], json!("no route configured for model 'unknown-model'"));
        assert_eq!(json["error"]["type"], json!("server_error"));
        assert_eq!(json["error"]["code"], json!("model_not_found"));
    }
}
