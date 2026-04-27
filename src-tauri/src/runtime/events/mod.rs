use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::application::dto::common::AppErrorDto;
use crate::application::dto::job::{JobKindDto, JobStatusDto};
use crate::domain::common::error::AppResult;
use crate::domain::common::ids::JobId;

pub const JOB_PROGRESS_EVENT: &str = "job:progress";
pub const JOB_FINISHED_EVENT: &str = "job:finished";

#[derive(Debug, Clone)]
pub enum AppEvent {
    JobProgress(JobProgressEvent),
    JobFinished(JobFinishedEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobProgressEvent {
    pub job_id: JobId,
    pub kind: JobKindDto,
    pub status: JobStatusDto,
    pub stage: String,
    pub progress_percent: Option<u8>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobFinishedEvent {
    pub job_id: JobId,
    pub kind: JobKindDto,
    pub status: JobStatusDto,
    pub stage: String,
    pub progress_percent: Option<u8>,
    pub message: Option<String>,
    pub error: Option<AppErrorDto>,
}

pub trait AppEventBus: Send + Sync {
    fn publish(&self, event: AppEvent) -> AppResult<()>;
}

#[derive(Debug, Default)]
pub struct NoopEventBus;

impl AppEventBus for NoopEventBus {
    fn publish(&self, _event: AppEvent) -> AppResult<()> {
        Ok(())
    }
}

pub type SharedEventBus = Arc<dyn AppEventBus>;
