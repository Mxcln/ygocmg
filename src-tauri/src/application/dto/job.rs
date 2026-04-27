use serde::{Deserialize, Serialize};

use crate::domain::common::ids::JobId;
use crate::domain::common::time::AppTimestamp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobKindDto {
    StandardPackIndexRebuild,
    ImportPack,
    ExportBundle,
    Test,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatusDto {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobAcceptedDto {
    pub job_id: JobId,
    pub kind: JobKindDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSnapshotDto {
    pub job_id: JobId,
    pub kind: JobKindDto,
    pub status: JobStatusDto,
    pub stage: String,
    pub progress_percent: Option<u8>,
    pub message: Option<String>,
    pub started_at: Option<AppTimestamp>,
    pub finished_at: Option<AppTimestamp>,
    pub error: Option<crate::application::dto::common::AppErrorDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetJobStatusInput {
    pub job_id: JobId,
}
