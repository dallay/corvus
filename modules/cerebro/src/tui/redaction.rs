use crate::config::TuiConfig;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;
use std::collections::HashSet;

const REDACTED: &str = "<redacted>";
const REDACTED_LARGE: &str = "<redacted:payload-too-large>";

static EMAIL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b").expect("email regex")
});
static JWT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\beyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\b").expect("jwt regex")
});
static HEX_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)\b[a-f0-9]{32,}\b").expect("hex"));
static BASE64_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b[A-Za-z0-9+/]{32,}={0,2}\b").expect("base64 regex"));
static TOKEN_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b[A-Za-z0-9_-]{24,}\b").expect("token regex"));

#[derive(Debug, Clone)]
pub struct RedactionPolicy {
    sensitive_fields: HashSet<String>,
    sensitive_word_patterns: Vec<Regex>,
    max_payload_bytes: usize,
}

impl RedactionPolicy {
    pub fn from_config(config: &TuiConfig) -> Self {
        let sensitive_fields = config
            .redact_fields
            .iter()
            .map(|field| field.trim().to_ascii_lowercase())
            .filter(|field| !field.is_empty())
            .collect();
        let sensitive_word_patterns = config
            .redact_fields
            .iter()
            .map(|field| field.trim())
            .filter(|field| !field.is_empty())
            .filter_map(|field| {
                let pattern = format!(r"(?i)\b{}\b\s*[:=]", regex::escape(field));
                Regex::new(&pattern).ok()
            })
            .collect();
        Self {
            sensitive_fields,
            sensitive_word_patterns,
            max_payload_bytes: config.max_payload_bytes.max(1),
        }
    }

    pub fn redact_with_allowlist(&self, value: &Value, allowlist: &[&str]) -> Option<Value> {
        let allowlist = allowlist
            .iter()
            .map(|field| field.trim().to_ascii_lowercase())
            .filter(|field| !field.is_empty())
            .collect::<HashSet<_>>();
        let redacted = self.redact_value(value, Some(&allowlist));
        Some(self.truncate_payload(redacted))
    }

    pub fn redact_observation(&self, value: &Value) -> Value {
        let allowlist = [
            "content", "what", "why", "where", "learned", "source", "tags",
        ];
        let allowlist = allowlist
            .iter()
            .map(|field| field.to_ascii_lowercase())
            .collect::<HashSet<_>>();
        self.truncate_payload(self.redact_value(value, Some(&allowlist)))
    }

    pub fn redact_text(&self, text: &str) -> String {
        if self
            .sensitive_word_patterns
            .iter()
            .any(|pattern| pattern.is_match(text))
            || contains_secret_pattern(text)
        {
            return REDACTED.to_string();
        }
        text.to_string()
    }

    fn redact_value(&self, value: &Value, allowlist: Option<&HashSet<String>>) -> Value {
        match value {
            Value::Object(map) => {
                let mut output = serde_json::Map::new();
                for (key, value) in map {
                    let normalized = key.trim().to_ascii_lowercase();
                    let allowed = allowlist
                        .map(|allow| allow.contains(&normalized))
                        .unwrap_or(false);
                    if self.sensitive_fields.contains(&normalized) || !allowed {
                        output.insert(key.clone(), Value::String(REDACTED.to_string()));
                        continue;
                    }
                    output.insert(key.clone(), self.redact_value(value, None));
                }
                Value::Object(output)
            }
            Value::Array(values) => Value::Array(
                values
                    .iter()
                    .map(|value| self.redact_value(value, None))
                    .collect(),
            ),
            Value::String(value) => Value::String(self.redact_text(value)),
            Value::Number(value) => Value::Number(value.clone()),
            Value::Bool(value) => Value::Bool(*value),
            Value::Null => Value::Null,
        }
    }

    fn truncate_payload(&self, value: Value) -> Value {
        match serde_json::to_vec(&value) {
            Ok(bytes) if bytes.len() <= self.max_payload_bytes => value,
            Ok(_) | Err(_) => Value::String(REDACTED_LARGE.to_string()),
        }
    }
}

fn contains_secret_pattern(text: &str) -> bool {
    EMAIL_RE.is_match(text)
        || JWT_RE.is_match(text)
        || HEX_RE.is_match(text)
        || BASE64_RE.is_match(text)
        || TOKEN_RE.is_match(text)
}
