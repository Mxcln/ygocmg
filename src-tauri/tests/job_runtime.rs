use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tempfile::tempdir;
use ygocmg_core::application::dto::job::{GetJobStatusInput, JobKindDto, JobStatusDto};
use ygocmg_core::bootstrap::wiring::build_app_state_with_event_bus;
use ygocmg_core::domain::common::error::{AppError, AppResult};
use ygocmg_core::presentation::commands::app_commands;
use ygocmg_core::runtime::events::{AppEvent, AppEventBus};

#[derive(Default)]
struct RecordingEventBus {
    events: Mutex<Vec<AppEvent>>,
}

impl RecordingEventBus {
    fn events(&self) -> Vec<AppEvent> {
        self.events.lock().unwrap().clone()
    }
}

impl AppEventBus for RecordingEventBus {
    fn publish(&self, event: AppEvent) -> AppResult<()> {
        self.events.lock().unwrap().push(event);
        Ok(())
    }
}

#[test]
fn job_runtime_reports_progress_and_success() {
    let app_dir = tempdir().unwrap();
    let event_bus = Arc::new(RecordingEventBus::default());
    let state =
        build_app_state_with_event_bus(app_dir.path().to_path_buf(), event_bus.clone()).unwrap();

    let accepted = state
        .jobs
        .submit(JobKindDto::Test, |ctx| {
            ctx.progress("scanning", Some(25), Some("Scanning inputs".to_string()))?;
            ctx.progress("writing", Some(75), Some("Writing outputs".to_string()))?;
            Ok(())
        })
        .unwrap();

    wait_for_status(&state, &accepted.job_id, JobStatusDto::Succeeded);
    let snapshot = app_commands::get_job_status(
        &state,
        GetJobStatusInput {
            job_id: accepted.job_id.clone(),
        },
    )
    .unwrap();

    assert_eq!(snapshot.kind, JobKindDto::Test);
    assert_eq!(snapshot.status, JobStatusDto::Succeeded);
    assert_eq!(snapshot.stage, "succeeded");
    assert_eq!(snapshot.progress_percent, Some(100));
    assert!(snapshot.started_at.is_some());
    assert!(snapshot.finished_at.is_some());
    assert!(snapshot.error.is_none());
    assert!(app_commands::list_active_jobs(&state).unwrap().is_empty());

    let events = event_bus.events();
    assert!(events.iter().any(|event| matches!(
        event,
        AppEvent::JobProgress(progress)
            if progress.job_id == accepted.job_id
                && progress.stage == "scanning"
                && progress.progress_percent == Some(25)
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        AppEvent::JobProgress(progress)
            if progress.job_id == accepted.job_id
                && progress.stage == "writing"
                && progress.progress_percent == Some(75)
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        AppEvent::JobFinished(finished)
            if finished.job_id == accepted.job_id
                && finished.status == JobStatusDto::Succeeded
                && finished.error.is_none()
    )));
}

#[test]
fn job_runtime_records_failed_jobs_and_errors() {
    let app_dir = tempdir().unwrap();
    let event_bus = Arc::new(RecordingEventBus::default());
    let state =
        build_app_state_with_event_bus(app_dir.path().to_path_buf(), event_bus.clone()).unwrap();

    let accepted = state
        .jobs
        .submit(JobKindDto::Test, |ctx| {
            ctx.progress("validating", Some(10), None)?;
            Err(AppError::new("job.test_failed", "test job failed"))
        })
        .unwrap();

    wait_for_status(&state, &accepted.job_id, JobStatusDto::Failed);
    let snapshot = app_commands::get_job_status(
        &state,
        GetJobStatusInput {
            job_id: accepted.job_id.clone(),
        },
    )
    .unwrap();

    assert_eq!(snapshot.status, JobStatusDto::Failed);
    assert_eq!(snapshot.stage, "failed");
    assert_eq!(snapshot.message.as_deref(), Some("test job failed"));
    let error = snapshot.error.unwrap();
    assert_eq!(error.code, "job.test_failed");
    assert_eq!(error.message, "test job failed");
    assert!(app_commands::list_active_jobs(&state).unwrap().is_empty());

    let events = event_bus.events();
    assert!(events.iter().any(|event| matches!(
        event,
        AppEvent::JobFinished(finished)
            if finished.job_id == accepted.job_id
                && finished.status == JobStatusDto::Failed
                && finished.error.as_ref().map(|error| error.code.as_str()) == Some("job.test_failed")
    )));
}

#[test]
fn active_jobs_lists_running_jobs_until_they_finish() {
    let app_dir = tempdir().unwrap();
    let event_bus = Arc::new(RecordingEventBus::default());
    let state = build_app_state_with_event_bus(app_dir.path().to_path_buf(), event_bus).unwrap();
    let (started_tx, started_rx) = mpsc::channel();
    let (finish_tx, finish_rx) = mpsc::channel();

    let accepted = state
        .jobs
        .submit(JobKindDto::Test, move |ctx| {
            ctx.progress("waiting", Some(5), None)?;
            started_tx.send(()).unwrap();
            finish_rx.recv().unwrap();
            Ok(())
        })
        .unwrap();

    started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let active = app_commands::list_active_jobs(&state).unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].job_id, accepted.job_id);
    assert_eq!(active[0].status, JobStatusDto::Running);

    finish_tx.send(()).unwrap();
    wait_for_status(&state, &accepted.job_id, JobStatusDto::Succeeded);
    assert!(app_commands::list_active_jobs(&state).unwrap().is_empty());
}

fn wait_for_status(state: &ygocmg_core::bootstrap::AppState, job_id: &str, expected: JobStatusDto) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let snapshot = app_commands::get_job_status(
            state,
            GetJobStatusInput {
                job_id: job_id.to_string(),
            },
        )
        .unwrap();
        if snapshot.status == expected {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for job status"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
