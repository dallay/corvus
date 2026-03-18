use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollectionReport {
    pub count: usize,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MigrationStatus {
    Ok,
    Mismatch,
    Error,
}

impl MigrationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Mismatch => "mismatch",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MigrationReport {
    pub source: String,
    pub target: String,
    pub collections: BTreeMap<String, CollectionReport>,
    pub status: MigrationStatus,
}

impl MigrationReport {
    pub fn to_json_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or_else(|_| serde_json::json!({ "status": "error" }))
    }

    pub fn to_human_string(&self) -> String {
        let mut output = format!(
            "Migration Report\nSource: {}\nTarget: {}\nStatus: {}\n",
            self.source,
            self.target,
            self.status.as_str()
        );
        for (collection, summary) in &self.collections {
            output.push_str(&format!(
                "- {}: {} records ({})\n",
                collection, summary.count, summary.checksum
            ));
        }
        output
    }
}
