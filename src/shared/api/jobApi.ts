import { invokeApi } from "./invoke";
import type { GetJobStatusInput, JobSnapshot } from "../contracts/job";

export const jobApi = {
  getJobStatus(input: GetJobStatusInput) {
    return invokeApi<JobSnapshot>("get_job_status", { input });
  },

  listActiveJobs() {
    return invokeApi<JobSnapshot[]>("list_active_jobs");
  },
};
