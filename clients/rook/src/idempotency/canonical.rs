#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{canonicalize_json, canonicalize_json_bytes, hash_canonical_json};

    #[test]
    fn canonical_json_treats_object_key_reordering_as_equivalent() {
        let left = json!({
            "model": "gpt-4o",
            "messages": [{"role": "user", "content": "Hello"}],
            "metadata": {"b": 2, "a": 1}
        });
        let right = json!({
            "metadata": {"a": 1, "b": 2},
            "messages": [{"content": "Hello", "role": "user"}],
            "model": "gpt-4o"
        });

        assert_eq!(canonicalize_json(&left), canonicalize_json(&right));
        assert_eq!(hash_canonical_json(&left), hash_canonical_json(&right));
    }

    #[test]
    fn canonical_json_treats_array_reordering_as_a_mismatch() {
        let left = json!({
            "model": "gpt-4o",
            "messages": [
                {"role": "system", "content": "You are helpful"},
                {"role": "user", "content": "Hello"}
            ]
        });
        let right = json!({
            "model": "gpt-4o",
            "messages": [
                {"role": "user", "content": "Hello"},
                {"role": "system", "content": "You are helpful"}
            ]
        });

        assert_ne!(canonicalize_json(&left), canonicalize_json(&right));
        assert_ne!(hash_canonical_json(&left), hash_canonical_json(&right));
    }

    #[test]
    fn canonical_json_hash_includes_unknown_passthrough_fields() {
        let with_passthrough = json!({
            "model": "gpt-4o",
            "messages": [{"role": "user", "content": "Hello"}],
            "logprobs": true
        });
        let without_passthrough = json!({
            "model": "gpt-4o",
            "messages": [{"role": "user", "content": "Hello"}]
        });

        assert_ne!(
            hash_canonical_json(&with_passthrough),
            hash_canonical_json(&without_passthrough)
        );
    }

    #[test]
    fn canonicalize_json_bytes_returns_stable_bytes_for_semantically_equal_payloads() {
        let left = br#"{"b":2,"a":1}"#;
        let right = br#"{"a":1,"b":2}"#;

        assert_eq!(
            canonicalize_json_bytes(left).expect("left payload should canonicalize"),
            canonicalize_json_bytes(right).expect("right payload should canonicalize")
        );
    }
}
use serde_json::{Map, Value};

pub fn canonicalize_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let sorted = map
                .iter()
                .map(|(key, value)| (key.clone(), canonicalize_json(value)))
                .collect::<Map<String, Value>>();
            Value::Object(sorted)
        }
        Value::Array(values) => Value::Array(values.iter().map(canonicalize_json).collect()),
        _ => value.clone(),
    }
}

pub fn canonicalize_json_bytes(raw: &[u8]) -> Result<Vec<u8>, serde_json::Error> {
    let value: Value = serde_json::from_slice(raw)?;
    serde_json::to_vec(&canonicalize_json(&value))
}

pub fn hash_canonical_json(value: &Value) -> String {
    let canonical = serde_json::to_vec(&canonicalize_json(value))
        .expect("canonical JSON serialization should not fail");
    hash_canonical_bytes(&canonical)
}

pub fn hash_canonical_bytes(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};

    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}
