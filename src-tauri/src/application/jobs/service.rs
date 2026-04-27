use crate::application::dto::job::{GetJobStatusInput, JobSnapshotDto};
use crate::bootstrap::AppState;
use crate::domain::common::error::AppResult;

pub struct JobService<'a> {
    state: &'a AppState,
}

impl<'a> JobService<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    pub fn get_job_status(&self, input: GetJobStatusInput) -> AppResult<JobSnapshotDto> {
        self.state.jobs.get_status(&input.job_id).map(Into::into)
    }

    pub fn list_active_jobs(&self) -> AppResult<Vec<JobSnapshotDto>> {
        self.state
            .jobs
            .list_active()
            .map(|jobs| jobs.into_iter().map(Into::into).collect())
    }
}
