use sha2::{Digest, Sha256};

pub fn canonical_json_checksum(value: &serde_json::Value) -> String {
    let canonical = canonical_json_string(value);
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    let digest = hasher.finalize();
    format!("sha256:{}", hex::encode(digest))
}

fn canonical_json_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::String(value) => serde_json::to_string(value)
            .unwrap_or_else(|_| format!("\"{}\"", value.replace('"', "\\\""))),
        serde_json::Value::Array(values) => {
            let mut output = String::from("[");
            for (idx, entry) in values.iter().enumerate() {
                if idx > 0 {
                    output.push(',');
                }
                output.push_str(&canonical_json_string(entry));
            }
            output.push(']');
            output
        }
        serde_json::Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            let mut output = String::from("{");
            for (idx, key) in keys.iter().enumerate() {
                if idx > 0 {
                    output.push(',');
                }
                output.push_str(
                    &serde_json::to_string(key)
                        .unwrap_or_else(|_| format!("\"{}\"", key.replace('"', "\\\""))),
                );
                output.push(':');
                if let Some(value) = map.get(*key) {
                    output.push_str(&canonical_json_string(value));
                } else {
                    output.push_str("null");
                }
            }
            output.push('}');
            output
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn canonical_json_string_orders_object_keys() {
        let value = json!({ "b": 2, "a": 1 });
        assert_eq!(canonical_json_string(&value), r#"{"a":1,"b":2}"#);
    }
}
