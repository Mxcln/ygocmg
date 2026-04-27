use serde::{Deserialize, Serialize};

use crate::domain::common::error::AppError;
use crate::domain::common::ids::{ConfirmationToken, PreviewToken};
use crate::domain::common::issue::ValidationIssue;
use crate::domain::common::time::AppTimestamp;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationFeedback {
    pub warnings: Vec<ValidationIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppResponse<T> {
    pub data: T,
    pub warnings: Vec<ValidationIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum WriteResultDto<T> {
    Ok {
        data: T,
        warnings: Vec<ValidationIssue>,
    },
    NeedsConfirmation {
        confirmation_token: ConfirmationToken,
        warnings: Vec<ValidationIssue>,
        preview: Option<serde_json::Value>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewResultDto<T> {
    pub preview_token: PreviewToken,
    pub snapshot_hash: String,
    pub expires_at: AppTimestamp,
    pub data: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppErrorDto {
    pub code: String,
    pub message: String,
    pub details: serde_json::Value,
}

impl From<AppError> for AppErrorDto {
    fn from(value: AppError) -> Self {
        Self {
            code: value.code,
            message: value.message,
            details: serde_json::to_value(value.details).unwrap_or(serde_json::Value::Null),
        }
    }
}
