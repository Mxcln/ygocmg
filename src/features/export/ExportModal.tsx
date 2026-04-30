import { useState, useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-dialog";
import { useShellStore } from "../../shared/stores/shellStore";
import { exportApi } from "../../shared/api/exportApi";
import { jobApi } from "../../shared/api/jobApi";
import { formatError, formatValidationIssue } from "../../shared/utils/format";
import {
  formatIssueDetail,
  formatIssueLevel,
  formatJobError,
  formatJobStage,
  formatJobStatus,
} from "../../shared/utils/messages";
import type { GlobalConfig } from "../../shared/contracts/config";
import type { ExportPreviewResult } from "../../shared/contracts/export";
import type { JobSnapshot } from "../../shared/contracts/job";
import { languageExists } from "../../shared/utils/language";
import shared from "../../shared/styles/shared.module.css";
import exportStyles from "./ExportModal.module.css";
import { TextLanguagePicker } from "../language/TextLanguagePicker";

type WizardStep = 1 | 2 | 3;

function isTerminalJob(job: JobSnapshot): boolean {
  return ["succeeded", "failed", "cancelled"].includes(job.status);
}

export interface ExportModalProps {
  config: GlobalConfig;
  onNotice: (tone: "success" | "warning" | "error", title: string, detail: string) => void;
}

export function ExportModal({ config, onNotice }: ExportModalProps) {
  const closeModal = useShellStore((s) => s.closeModal);
  const workspaceId = useShellStore((s) => s.workspaceId);
  const openPackIds = useShellStore((s) => s.openPackIds);
  const packMetadataMap = useShellStore((s) => s.packMetadataMap);

  const [step, setStep] = useState<WizardStep>(1);
  const [selectedPackIds, setSelectedPackIds] = useState<string[]>([]);
  const [exportLanguage, setExportLanguage] = useState("");
  const [outputDir, setOutputDir] = useState("");
  const [outputName, setOutputName] = useState("");

  const [busy, setBusy] = useState<string | null>(null);
  const [previewResult, setPreviewResult] = useState<ExportPreviewResult | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [activeJobId, setActiveJobId] = useState<string | null>(null);
  const [lastJob, setLastJob] = useState<JobSnapshot | null>(null);

  const jobQuery = useQuery({
    queryKey: ["export-job", activeJobId],
    queryFn: () => jobApi.getJobStatus({ jobId: activeJobId! }),
    enabled: activeJobId !== null,
    refetchInterval: activeJobId ? 700 : false,
  });

  const activeJob = jobQuery.data ?? null;

  useEffect(() => {
    if (!activeJobId || !activeJob || !isTerminalJob(activeJob)) return;
    setLastJob(activeJob);
    setActiveJobId(null);
  }, [activeJobId, activeJob]);

  useEffect(() => {
    if (selectedPackIds.length === 0) {
      setExportLanguage("");
      return;
    }
    const defaults = selectedPackIds
      .map((packId) => packMetadataMap[packId]?.default_export_language ?? "")
      .filter((language) => languageExists(config.text_language_catalog, language));
    const commonDefault = defaults.length === selectedPackIds.length && defaults.every((language) => language === defaults[0])
      ? defaults[0]
      : "";
    setExportLanguage(commonDefault);
  }, [selectedPackIds, packMetadataMap, config.text_language_catalog]);

  function handleTogglePack(packId: string) {
    setSelectedPackIds((prev) =>
      prev.includes(packId) ? prev.filter((id) => id !== packId) : [...prev, packId],
    );
  }

  function handleSelectAll() {
    if (selectedPackIds.length === openPackIds.length) {
      setSelectedPackIds([]);
    } else {
      setSelectedPackIds([...openPackIds]);
    }
  }

  async function handleBrowseOutputDir() {
    const selected = await open({ directory: true, multiple: false });
    if (!selected) return;
    setOutputDir(typeof selected === "string" ? selected : selected);
  }

  const canPreview =
    selectedPackIds.length > 0 &&
    exportLanguage.trim() !== "" &&
    outputDir.trim() !== "" &&
    outputName.trim() !== "";

  async function handlePreview() {
    if (!workspaceId || !canPreview) return;

    setBusy("preview");
    setPreviewError(null);
    setPreviewResult(null);
    setLastJob(null);

    try {
      const result = await exportApi.previewExportBundle({
        workspaceId,
        packIds: selectedPackIds,
        exportLanguage: exportLanguage.trim(),
        outputDir: outputDir.trim(),
        outputName: outputName.trim(),
      });
      setPreviewResult(result);
      setStep(2);
    } catch (err) {
      setPreviewError(formatError(err));
    } finally {
      setBusy(null);
    }
  }

  async function handleExecute() {
    if (!previewResult) return;
    setBusy("execute");

    try {
      const job = await exportApi.executeExportBundle({
        previewToken: previewResult.preview_token,
      });
      setActiveJobId(job.job_id);
      setStep(3);
    } catch (err) {
      onNotice("error", "Export failed", formatError(err));
    } finally {
      setBusy(null);
    }
  }

  function handleBackFromStep2() {
    setPreviewResult(null);
    setPreviewError(null);
    setLastJob(null);
    setActiveJobId(null);
    setStep(1);
  }

  function handleBackFromStep3() {
    setLastJob(null);
    setActiveJobId(null);
    setStep(2);
  }

  const exporting = activeJobId !== null;
  const jobDone = lastJob !== null && isTerminalJob(lastJob);
  const jobSucceeded = lastJob?.status === "succeeded";
  const jobFailed = lastJob?.status === "failed";
  const displayJob = activeJob ?? lastJob;

  return (
    <>
      <header className={shared.modalHeader}>
        <h2>Export Expansions</h2>
        <div className={exportStyles.exportHeaderRight}>
          <div className={shared.importWizardSteps}>
            <span className={`${shared.wizardStep} ${step >= 1 ? "active" : ""} ${step === 1 ? "current" : ""}`}>
              1. Configure
            </span>
            <span className={shared.wizardStepSep}>&rsaquo;</span>
            <span className={`${shared.wizardStep} ${step >= 2 ? "active" : ""} ${step === 2 ? "current" : ""}`}>
              2. Preview
            </span>
            <span className={shared.wizardStepSep}>&rsaquo;</span>
            <span className={`${shared.wizardStep} ${step >= 3 ? "active" : ""} ${step === 3 ? "current" : ""}`}>
              3. Export
            </span>
          </div>
          <button className={shared.modalCloseButton} type="button" onClick={closeModal}>
            Close
          </button>
        </div>
      </header>

      <div className={`${shared.modalBody} ${exportStyles.exportModalBody}`}>
        {step === 1 && (
          <div className={`${shared.formStack} ${exportStyles.exportForm}`}>
            <div className={shared.field}>
              <span>
                Select packs to export
                {openPackIds.length > 0 && (
                  <button
                    type="button"
                    className={`${shared.ghostButton} ${exportStyles.exportSelectAllBtn}`}
                    onClick={handleSelectAll}
                  >
                    {selectedPackIds.length === openPackIds.length ? "Deselect All" : "Select All"}
                  </button>
                )}
              </span>
              {openPackIds.length === 0 ? (
                <div className={exportStyles.exportEmptyPacks}>No packs are currently open.</div>
              ) : (
                <div className={exportStyles.exportPackList}>
                  {openPackIds.map((packId) => {
                    const meta = packMetadataMap[packId];
                    const checked = selectedPackIds.includes(packId);
                    return (
                      <label key={packId} className={`${exportStyles.exportPackItem} ${checked ? "selected" : ""}`}>
                        <input
                          type="checkbox"
                          checked={checked}
                          onChange={() => handleTogglePack(packId)}
                        />
                        <span className={exportStyles.exportPackName}>{meta?.name ?? packId}</span>
                        {meta && (
                          <span className={exportStyles.exportPackDetail}>
                            {meta.author} &middot; v{meta.version}
                          </span>
                        )}
                      </label>
                    );
                  })}
                </div>
              )}
            </div>

            <div className={shared.field}>
              <span>Export language (required)</span>
              <TextLanguagePicker
                catalog={config.text_language_catalog}
                value={exportLanguage}
                onChange={setExportLanguage}
                allowEmpty
                placeholder="Select export language"
              />
            </div>

            <div className={shared.field}>
              <span>Output directory (required)</span>
              <div className={shared.filePickerRow}>
                <input
                  readOnly
                  value={outputDir}
                  placeholder="Select output directory..."
                  title={outputDir || undefined}
                />
                <button
                  type="button"
                  className={shared.ghostButton}
                  onClick={() => void handleBrowseOutputDir()}
                >
                  Browse
                </button>
                {outputDir && (
                  <button
                    type="button"
                    className={shared.ghostButton}
                    onClick={() => setOutputDir("")}
                  >
                    Clear
                  </button>
                )}
              </div>
            </div>

            <div className={shared.field}>
              <span>Output name (required)</span>
              <input
                value={outputName}
                onChange={(e) => setOutputName(e.target.value)}
                placeholder="my-expansion"
              />
              {outputDir && outputName.trim() && (
                <span className={exportStyles.exportOutputPreview}>
                  Output: {outputDir}
                  {outputDir.endsWith("\\") || outputDir.endsWith("/") ? "" : "\\"}
                  {outputName.trim()}
                </span>
              )}
            </div>

            {previewError && <div className={shared.importErrorBanner}>{previewError}</div>}

            <div className={shared.formActions}>
              <button type="button" className={shared.ghostButton} onClick={closeModal}>
                Cancel
              </button>
              <button
                type="button"
                className={shared.primaryButton}
                disabled={!canPreview || busy !== null}
                onClick={() => void handlePreview()}
              >
                {busy === "preview" ? "Previewing..." : "Preview Export"}
              </button>
            </div>
          </div>
        )}

        {step === 2 && previewResult && (
          <div className={`${shared.importPreviewStep} ${exportStyles.exportForm}`}>
            <div className={shared.importPreviewSummary}>
              <div className={shared.importStat}>
                <span className={shared.importStatValue}>{previewResult.data.pack_count}</span>
                <span className={shared.importStatLabel}>Packs</span>
              </div>
              <div className={shared.importStat}>
                <span className={shared.importStatValue}>{previewResult.data.card_count}</span>
                <span className={shared.importStatLabel}>Cards</span>
              </div>
              <div className={shared.importStat}>
                <span className={shared.importStatValue}>{previewResult.data.main_image_count}</span>
                <span className={shared.importStatLabel}>Images</span>
              </div>
              <div className={shared.importStat}>
                <span className={shared.importStatValue}>{previewResult.data.field_image_count}</span>
                <span className={shared.importStatLabel}>Field Imgs</span>
              </div>
              <div className={shared.importStat}>
                <span className={shared.importStatValue}>{previewResult.data.script_count}</span>
                <span className={shared.importStatLabel}>Scripts</span>
              </div>
              <div className={shared.importStat}>
                <span
                  className={shared.importStatValue}
                  data-level={previewResult.data.error_count > 0 ? "error" : undefined}
                >
                  {previewResult.data.error_count}
                </span>
                <span className={shared.importStatLabel}>Errors</span>
              </div>
              <div className={shared.importStat}>
                <span
                  className={shared.importStatValue}
                  data-level={previewResult.data.warning_count > 0 ? "warning" : undefined}
                >
                  {previewResult.data.warning_count}
                </span>
                <span className={shared.importStatLabel}>Warnings</span>
              </div>
            </div>

            {previewResult.data.error_count > 0 && (
              <div className={shared.importErrorBanner}>
                Export has {previewResult.data.error_count} blocking error
                {previewResult.data.error_count > 1 ? "s" : ""}. Fix the issues and try again.
              </div>
            )}

            {previewResult.data.issues.length > 0 && (
              <div className={shared.importIssuesList}>
                <strong className={shared.importIssuesHeader}>
                  Issues ({previewResult.data.issues.length})
                </strong>
                <ul>
                  {previewResult.data.issues.map((issue, idx) => (
                    <li key={idx} className={shared.importIssue} data-level={issue.level}>
                      <span className={shared.importIssueBadge}>{formatIssueLevel(issue.level)}</span>
                      <span className={shared.importIssueMessage}>{formatValidationIssue(issue)}</span>
                      {formatIssueDetail(issue) && (
                        <span className={shared.importIssueParams}>{formatIssueDetail(issue)}</span>
                      )}
                    </li>
                  ))}
                </ul>
              </div>
            )}

            <div className={shared.formActions}>
              <button type="button" className={shared.ghostButton} onClick={handleBackFromStep2}>
                Back
              </button>
              <button
                type="button"
                className={shared.primaryButton}
                disabled={previewResult.data.error_count > 0 || busy !== null}
                onClick={() => void handleExecute()}
              >
                {busy === "execute" ? "Submitting..." : "Export"}
              </button>
            </div>
          </div>
        )}

        {step === 3 && (
          <div className={`${shared.importPreviewStep} ${exportStyles.exportForm}`}>
            {previewResult && (
              <div className={shared.importPreviewSummary}>
                <div className={shared.importStat}>
                  <span className={shared.importStatValue}>{previewResult.data.pack_count}</span>
                  <span className={shared.importStatLabel}>Packs</span>
                </div>
                <div className={shared.importStat}>
                  <span className={shared.importStatValue}>{previewResult.data.card_count}</span>
                  <span className={shared.importStatLabel}>Cards</span>
                </div>
                <div className={shared.importStat}>
                  <span className={shared.importStatValue}>{previewResult.data.main_image_count}</span>
                  <span className={shared.importStatLabel}>Images</span>
                </div>
                <div className={shared.importStat}>
                  <span className={shared.importStatValue}>{previewResult.data.field_image_count}</span>
                  <span className={shared.importStatLabel}>Field Imgs</span>
                </div>
                <div className={shared.importStat}>
                  <span className={shared.importStatValue}>{previewResult.data.script_count}</span>
                  <span className={shared.importStatLabel}>Scripts</span>
                </div>
              </div>
            )}

            {displayJob && (
              <div className={shared.importJobStrip} data-status={displayJob.status}>
                <span className={shared.importJobStatus}>{formatJobStatus(displayJob.status)}</span>
                <strong>{formatJobStage(displayJob.stage)}</strong>
                {displayJob.progress_percent != null && (
                  <span>{displayJob.progress_percent}%</span>
                )}
                {formatJobError(displayJob) && (
                  <span className={shared.importJobError}>{formatJobError(displayJob)}</span>
                )}
              </div>
            )}

            {jobSucceeded && (
              <div className={shared.importSuccessBanner}>
                Export completed successfully.
                {outputDir && outputName.trim() && (
                  <span className={exportStyles.exportOutputPath}>
                    Output: {outputDir}
                    {outputDir.endsWith("\\") || outputDir.endsWith("/") ? "" : "\\"}
                    {outputName.trim()}
                  </span>
                )}
              </div>
            )}

            <div className={shared.formActions}>
              {exporting && (
                <button type="button" className={shared.ghostButton} disabled>
                  Exporting...
                </button>
              )}

              {jobSucceeded && (
                <button type="button" className={shared.primaryButton} onClick={closeModal}>
                  Done
                </button>
              )}

              {jobFailed && (
                <>
                  <button type="button" className={shared.ghostButton} onClick={handleBackFromStep3}>
                    Back
                  </button>
                  <span className={shared.importFailHint}>
                    Export job failed. Check errors above and try again.
                  </span>
                </>
              )}

              {!exporting && !jobDone && (
                <button type="button" className={shared.ghostButton} onClick={handleBackFromStep3}>
                  Back
                </button>
              )}
            </div>
          </div>
        )}
      </div>
    </>
  );
}
