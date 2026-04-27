import type { JobId } from "./common";
import type { AppError } from "../api/invoke";

export type JobKind = "standard_pack_index_rebuild" | "import_pack" | "export_bundle" | "test";

export type JobStatus = "pending" | "running" | "succeeded" | "failed" | "cancelled";

export interface JobAccepted {
  job_id: JobId;
  kind: JobKind;
}

export interface JobSnapshot {
  job_id: JobId;
  kind: JobKind;
  status: JobStatus;
  stage: string;
  progress_percent: number | null;
  message: string | null;
  started_at: string | null;
  finished_at: string | null;
  error: AppError | null;
}

export interface JobProgressEvent {
  job_id: JobId;
  kind: JobKind;
  status: JobStatus;
  stage: string;
  progress_percent: number | null;
  message: string | null;
}

export interface JobFinishedEvent {
  job_id: JobId;
  kind: JobKind;
  status: JobStatus;
  stage: string;
  progress_percent: number | null;
  message: string | null;
  error: AppError | null;
}

export interface GetJobStatusInput {
  jobId: JobId;
}
