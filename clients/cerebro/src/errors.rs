use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CerebroErrorCode {
    Validation,
    Unauthorized,
    Forbidden,
    NotImplemented,
    NotFound,
    Conflict,
    Storage,
    Internal,
}

impl CerebroErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Validation => "validation_error",
            Self::Unauthorized => "unauthorized",
            Self::Forbidden => "forbidden",
            Self::NotImplemented => "not_implemented",
            Self::NotFound => "not_found",
            Self::Conflict => "conflict",
            Self::Storage => "storage_error",
            Self::Internal => "internal_error",
        }
    }

    pub fn as_i64(&self) -> i64 {
        match self {
            Self::Validation => -32000,
            Self::Unauthorized => -32001,
            Self::Forbidden => -32003,
            Self::NotImplemented => -32004,
            Self::NotFound => -32005,
            Self::Conflict => -32006,
            Self::Storage => -32010,
            Self::Internal => -32603,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CerebroErrorResponse {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

#[derive(Debug, Error)]
pub enum CerebroError {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("not implemented: {0}")]
    NotImplemented(String),
    #[error("not found")]
    NotFound,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("storage error: {0}")]
    Storage(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl CerebroError {
    pub fn code(&self) -> CerebroErrorCode {
        match self {
            Self::Validation(_) => CerebroErrorCode::Validation,
            Self::Unauthorized => CerebroErrorCode::Unauthorized,
            Self::Forbidden(_) => CerebroErrorCode::Forbidden,
            Self::NotImplemented(_) => CerebroErrorCode::NotImplemented,
            Self::NotFound => CerebroErrorCode::NotFound,
            Self::Conflict(_) => CerebroErrorCode::Conflict,
            Self::Storage(_) => CerebroErrorCode::Storage,
            Self::Internal(_) => CerebroErrorCode::Internal,
        }
    }

    pub fn to_response(&self) -> CerebroErrorResponse {
        CerebroErrorResponse {
            code: self.code().as_i64(),
            message: self.to_string(),
            details: None,
        }
    }
}
