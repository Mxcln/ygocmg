use std::collections::BTreeMap;
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, RwLock};

use uuid::Uuid;

use crate::application::dto::common::AppErrorDto;
use crate::application::dto::job::{JobAcceptedDto, JobKindDto, JobSnapshotDto, JobStatusDto};
use crate::domain::common::error::{AppError, AppResult};
use crate::domain::common::ids::JobId;
use crate::domain::common::time::{AppTimestamp, now_utc};
use crate::runtime::events::{AppEvent, JobFinishedEvent, JobProgressEvent, SharedEventBus};

const INITIAL_STAGE: &str = "pending";
const RUNNING_STAGE: &str = "running";
const SUCCEEDED_STAGE: &str = "succeeded";
const FAILED_STAGE: &str = "failed";

#[derive(Debug, Clone)]
pub struct JobSnapshot {
    pub job_id: JobId,
    pub kind: JobKindDto,
    pub status: JobStatusDto,
    pub stage: String,
    pub progress_percent: Option<u8>,
    pub message: Option<String>,
    pub started_at: Option<AppTimestamp>,
    pub finished_at: Option<AppTimestamp>,
    pub error: Option<AppError>,
}

#[derive(Debug, Default)]
pub struct JobStore {
    jobs: BTreeMap<JobId, JobSnapshot>,
}

impl JobStore {
    pub fn insert(&mut self, snapshot: JobSnapshot) {
        debug_assert!(
            !self.jobs.contains_key(&snapshot.job_id),
            "job store insert called with an existing job id"
        );
        self.jobs.insert(snapshot.job_id.clone(), snapshot);
    }

    pub fn get(&self, job_id: &JobId) -> Option<JobSnapshot> {
        self.jobs.get(job_id).cloned()
    }

    pub fn update(&mut self, snapshot: JobSnapshot) {
        debug_assert!(
            self.jobs.contains_key(&snapshot.job_id),
            "job store update called with a missing job id"
        );
        self.jobs.insert(snapshot.job_id.clone(), snapshot);
    }

    pub fn list_active(&self) -> Vec<JobSnapshot> {
        self.jobs
            .values()
            .filter(|snapshot| {
                matches!(
                    snapshot.status,
                    JobStatusDto::Pending | JobStatusDto::Running
                )
            })
            .cloned()
            .collect()
    }
}

#[derive(Clone)]
pub struct JobRuntime {
    store: Arc<RwLock<JobStore>>,
    event_bus: SharedEventBus,
}

impl JobRuntime {
    pub fn new(event_bus: SharedEventBus) -> Self {
        Self {
            store: Arc::new(RwLock::new(JobStore::default())),
            event_bus,
        }
    }

    pub fn submit<F>(&self, kind: JobKindDto, runner: F) -> AppResult<JobAcceptedDto>
    where
        F: FnOnce(JobContext) -> AppResult<()> + Send + 'static,
    {
        let job_id = Uuid::now_v7().to_string();
        let snapshot = JobSnapshot {
            job_id: job_id.clone(),
            kind,
            status: JobStatusDto::Pending,
            stage: INITIAL_STAGE.to_string(),
            progress_percent: None,
            message: None,
            started_at: None,
            finished_at: None,
            error: None,
        };
        self.write_store()?.insert(snapshot);

        let context = JobContext {
            job_id: job_id.clone(),
            store: Arc::clone(&self.store),
            event_bus: Arc::clone(&self.event_bus),
        };

        tauri::async_runtime::spawn_blocking(move || {
            let run_result = catch_unwind(AssertUnwindSafe(|| {
                context.mark_running()?;
                runner(context.clone())
            }));

            match run_result {
                Ok(Ok(())) => context.succeed(),
                Ok(Err(error)) => context.fail(error),
                Err(_) => context.fail(AppError::new(
                    "job.panic",
                    "job runner panicked during execution",
                )),
            }
        });

        Ok(JobAcceptedDto { job_id, kind })
    }

    pub fn get_status(&self, job_id: &JobId) -> AppResult<JobSnapshot> {
        self.read_store()?.get(job_id).ok_or_else(|| {
            AppError::new("job.not_found", "job was not found").with_detail("job_id", job_id)
        })
    }

    pub fn list_active(&self) -> AppResult<Vec<JobSnapshot>> {
        Ok(self.read_store()?.list_active())
    }

    fn read_store(&self) -> AppResult<std::sync::RwLockReadGuard<'_, JobStore>> {
        self.store
            .read()
            .map_err(|_| AppError::new("job.store_lock_poisoned", "job store lock poisoned"))
    }

    fn write_store(&self) -> AppResult<std::sync::RwLockWriteGuard<'_, JobStore>> {
        self.store
            .write()
            .map_err(|_| AppError::new("job.store_lock_poisoned", "job store lock poisoned"))
    }
}

impl fmt::Debug for JobRuntime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let job_count = self.store.read().map(|store| store.jobs.len()).ok();
        formatter
            .debug_struct("JobRuntime")
            .field("job_count", &job_count)
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
pub struct JobContext {
    job_id: JobId,
    store: Arc<RwLock<JobStore>>,
    event_bus: SharedEventBus,
}

impl JobContext {
    pub fn progress(
        &self,
        stage: impl Into<String>,
        progress_percent: Option<u8>,
        message: Option<String>,
    ) -> AppResult<()> {
        let mut snapshot = self.current_snapshot()?;
        snapshot.status = JobStatusDto::Running;
        if snapshot.started_at.is_none() {
            snapshot.started_at = Some(now_utc());
        }
        snapshot.stage = stage.into();
        snapshot.progress_percent = progress_percent.map(|value| value.min(100));
        snapshot.message = message;
        snapshot.error = None;
        self.persist(snapshot.clone())?;
        self.publish_progress(&snapshot);
        Ok(())
    }

    fn mark_running(&self) -> AppResult<()> {
        self.progress(RUNNING_STAGE, Some(0), None)
    }

    fn succeed(&self) {
        let result = (|| -> AppResult<()> {
            let mut snapshot = self.current_snapshot()?;
            snapshot.status = JobStatusDto::Succeeded;
            snapshot.stage = SUCCEEDED_STAGE.to_string();
            snapshot.progress_percent = Some(100);
            snapshot.finished_at = Some(now_utc());
            snapshot.error = None;
            self.persist(snapshot.clone())?;
            self.publish_finished(&snapshot);
            Ok(())
        })();
        if result.is_err() {
            // There is no caller for background completion; keep best-effort semantics.
        }
    }

    fn fail(&self, error: AppError) {
        let result = (|| -> AppResult<()> {
            let mut snapshot = self.current_snapshot()?;
            snapshot.status = JobStatusDto::Failed;
            snapshot.stage = FAILED_STAGE.to_string();
            snapshot.finished_at = Some(now_utc());
            snapshot.message = Some(error.message.clone());
            snapshot.error = Some(error);
            self.persist(snapshot.clone())?;
            self.publish_finished(&snapshot);
            Ok(())
        })();
        if result.is_err() {
            // There is no caller for background completion; keep best-effort semantics.
        }
    }

    fn current_snapshot(&self) -> AppResult<JobSnapshot> {
        self.store
            .read()
            .map_err(|_| AppError::new("job.store_lock_poisoned", "job store lock poisoned"))?
            .get(&self.job_id)
            .ok_or_else(|| {
                AppError::new("job.not_found", "job was not found")
                    .with_detail("job_id", &self.job_id)
            })
    }

    fn persist(&self, snapshot: JobSnapshot) -> AppResult<()> {
        self.store
            .write()
            .map_err(|_| AppError::new("job.store_lock_poisoned", "job store lock poisoned"))?
            .update(snapshot);
        Ok(())
    }

    fn publish_progress(&self, snapshot: &JobSnapshot) {
        let _ = self
            .event_bus
            .publish(AppEvent::JobProgress(JobProgressEvent {
                job_id: snapshot.job_id.clone(),
                kind: snapshot.kind,
                status: snapshot.status,
                stage: snapshot.stage.clone(),
                progress_percent: snapshot.progress_percent,
                message: snapshot.message.clone(),
            }));
    }

    fn publish_finished(&self, snapshot: &JobSnapshot) {
        let _ = self
            .event_bus
            .publish(AppEvent::JobFinished(JobFinishedEvent {
                job_id: snapshot.job_id.clone(),
                kind: snapshot.kind,
                status: snapshot.status,
                stage: snapshot.stage.clone(),
                progress_percent: snapshot.progress_percent,
                message: snapshot.message.clone(),
                error: snapshot.error.clone().map(AppErrorDto::from),
            }));
    }
}

impl From<JobSnapshot> for JobSnapshotDto {
    fn from(value: JobSnapshot) -> Self {
        Self {
            job_id: value.job_id,
            kind: value.kind,
            status: value.status,
            stage: value.stage,
            progress_percent: value.progress_percent,
            message: value.message,
            started_at: value.started_at,
            finished_at: value.finished_at,
            error: value.error.map(AppErrorDto::from),
        }
    }
}
