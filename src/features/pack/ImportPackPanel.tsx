import { useState, useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-dialog";
import { importApi } from "../../shared/api/importApi";
import { packApi } from "../../shared/api/packApi";
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
import type { PackMetadata } from "../../shared/contracts/pack";
import type { ImportPreviewResult } from "../../shared/contracts/import";
import type { JobSnapshot } from "../../shared/contracts/job";
import { preferredImportSourceLanguage } from "../../shared/utils/language";
import { useAppI18n } from "../../shared/i18n";
import shared from "../../shared/styles/shared.module.css";
import styles from "./ImportPackPanel.module.css";
import { LanguageOrderEditor } from "../language/LanguageOrderEditor";
import { TextLanguagePicker } from "../language/TextLanguagePicker";

type WizardStep = 1 | 2 | 3;

interface SourceForm {
  cdbPath: string;
  sourceLanguage: string;
  picsDir: string;
  fieldPicsDir: string;
  scriptDir: string;
  stringsConfPath: string;
}

interface MetadataForm {
  name: string;
  author: string;
  version: string;
  description: string;
  displayLanguageOrder: string[];
  defaultExportLanguage: string;
}

function emptySource(config: GlobalConfig): SourceForm {
  return {
    cdbPath: "",
    sourceLanguage: preferredImportSourceLanguage(config),
    picsDir: "",
    fieldPicsDir: "",
    scriptDir: "",
    stringsConfPath: "",
  };
}

const EMPTY_METADATA: MetadataForm = {
  name: "",
  author: "",
  version: "1.0.0",
  description: "",
  displayLanguageOrder: [],
  defaultExportLanguage: "",
};

function inferResourcePaths(cdbPath: string) {
  const sep = cdbPath.includes("\\") ? "\\" : "/";
  const lastSep = cdbPath.lastIndexOf(sep);
  const parentDir = lastSep >= 0 ? cdbPath.substring(0, lastSep) : "";
  const fileName = cdbPath.substring(lastSep + 1);
  const baseName = fileName.replace(/\.cdb$/i, "");
  return {
    suggestedName: baseName,
    picsDir: parentDir + sep + "pics",
    fieldPicsDir: parentDir + sep + "pics" + sep + "field",
    scriptDir: parentDir + sep + "script",
    stringsConfPath: parentDir + sep + "strings.conf",
  };
}

function isTerminalJob(job: JobSnapshot): boolean {
  return ["succeeded", "failed", "cancelled"].includes(job.status);
}

export interface ImportPackPanelProps {
  workspaceId: string;
  config: GlobalConfig;
  onPackOpened: (packId: string, metadata: PackMetadata) => void;
  onNotice: (tone: "success" | "warning" | "error", title: string, detail: string) => void;
  closeModal: () => void;
}

export function ImportPackPanel({
  workspaceId,
  config,
  onPackOpened,
  onNotice,
  closeModal,
}: ImportPackPanelProps) {
  const { t, td } = useAppI18n();
  const [step, setStep] = useState<WizardStep>(1);
  const [sourceForm, setSourceForm] = useState<SourceForm>(() => emptySource(config));
  const [metadataForm, setMetadataForm] = useState<MetadataForm>(EMPTY_METADATA);
  const [metadataInitialized, setMetadataInitialized] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);
  const [previewResult, setPreviewResult] = useState<ImportPreviewResult | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [activeJobId, setActiveJobId] = useState<string | null>(null);
  const [lastJob, setLastJob] = useState<JobSnapshot | null>(null);
  const [openingPack, setOpeningPack] = useState(false);

  const jobQuery = useQuery({
    queryKey: ["import-job", activeJobId],
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

  const canGoNext = sourceForm.cdbPath !== "" && sourceForm.sourceLanguage.trim() !== "";

  function handleGoToStep2() {
    if (!metadataInitialized) {
      const inferred = sourceForm.cdbPath ? inferResourcePaths(sourceForm.cdbPath) : null;
      setMetadataForm((prev) => ({
        ...prev,
        name: prev.name || inferred?.suggestedName || "",
        displayLanguageOrder:
          prev.displayLanguageOrder.length > 0 ? prev.displayLanguageOrder : [sourceForm.sourceLanguage],
        defaultExportLanguage: prev.defaultExportLanguage || sourceForm.sourceLanguage,
      }));
      setMetadataInitialized(true);
    }
    setStep(2);
  }

  async function handleSelectCdb() {
    const selected = await open({
      directory: false,
      multiple: false,
      filters: [{ name: "CDB", extensions: ["cdb"] }],
    });
    if (!selected) return;
    const path = typeof selected === "string" ? selected : selected;
    const inferred = inferResourcePaths(path);
    setSourceForm({
      ...sourceForm,
      cdbPath: path,
      picsDir: inferred.picsDir,
      fieldPicsDir: inferred.fieldPicsDir,
      scriptDir: inferred.scriptDir,
      stringsConfPath: inferred.stringsConfPath,
    });
    setMetadataInitialized(false);
  }

  async function handleBrowseDir(field: "picsDir" | "fieldPicsDir" | "scriptDir") {
    const selected = await open({ directory: true, multiple: false });
    if (!selected) return;
    const path = typeof selected === "string" ? selected : selected;
    setSourceForm({ ...sourceForm, [field]: path });
  }

  async function handleBrowseFile(field: "stringsConfPath") {
    const selected = await open({
      directory: false,
      multiple: false,
      filters: [{ name: "Strings", extensions: ["conf"] }],
    });
    if (!selected) return;
    const path = typeof selected === "string" ? selected : selected;
    setSourceForm({ ...sourceForm, [field]: path });
  }

  async function handlePreview() {
    const name = metadataForm.name.trim();
    const author = metadataForm.author.trim();
    const version = metadataForm.version.trim();
    if (!name || !author || !version) {
      onNotice("warning", td("pack.missingFields.title", "Missing fields"), td("import.missingFields.detail", "Pack name, author, and version are required."));
      return;
    }

    setBusy("preview");
    setPreviewError(null);
    setPreviewResult(null);
    setLastJob(null);

    try {
      const langOrder = metadataForm.displayLanguageOrder;

      const result = await importApi.previewImportPack({
        workspaceId,
        newPackName: name,
        newPackAuthor: author,
        newPackVersion: version,
        newPackDescription: metadataForm.description.trim() || null,
        displayLanguageOrder: langOrder,
        defaultExportLanguage: metadataForm.defaultExportLanguage.trim() || null,
        cdbPath: sourceForm.cdbPath,
        picsDir: sourceForm.picsDir || null,
        fieldPicsDir: sourceForm.fieldPicsDir || null,
        scriptDir: sourceForm.scriptDir || null,
        stringsConfPath: sourceForm.stringsConfPath || null,
        sourceLanguage: sourceForm.sourceLanguage.trim(),
      });

      setPreviewResult(result);
      setStep(3);
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
      const job = await importApi.executeImportPack({
        previewToken: previewResult.preview_token,
      });
      setActiveJobId(job.job_id);
    } catch (err) {
      onNotice("error", td("import.failed", "Import failed"), formatError(err));
    } finally {
      setBusy(null);
    }
  }

  async function handleOpenImportedPack() {
    if (!previewResult) return;
    setOpeningPack(true);
    try {
      const metadata = await packApi.openPack({ packId: previewResult.data.target_pack_id });
      onPackOpened(previewResult.data.target_pack_id, metadata);
      onNotice("success", td("import.imported.title", "Pack imported"), td("import.imported.detail", '"{name}" has been imported and opened.', { name: previewResult.data.target_pack_name }));
      closeModal();
    } catch (err) {
      onNotice("error", td("pack.openFailed", "Failed to open pack"), formatError(err));
    } finally {
      setOpeningPack(false);
    }
  }

  function handleBackFromStep3() {
    setPreviewResult(null);
    setPreviewError(null);
    setLastJob(null);
    setActiveJobId(null);
    setStep(2);
  }

  const importing = activeJobId !== null;
  const jobDone = lastJob !== null && isTerminalJob(lastJob);
  const jobSucceeded = lastJob?.status === "succeeded";
  const jobFailed = lastJob?.status === "failed";
  const displayJob = activeJob ?? lastJob;

  return (
    <section className={styles.importPanel}>
      <div className={shared.panelHeader}>
        <div className={shared.importWizardSteps}>
          <span className={`${shared.wizardStep} ${step >= 1 ? "active" : ""} ${step === 1 ? "current" : ""}`}>1. {td("import.step.source", "Source")}</span>
          <span className={shared.wizardStepSep}>&rsaquo;</span>
          <span className={`${shared.wizardStep} ${step >= 2 ? "active" : ""} ${step === 2 ? "current" : ""}`}>2. {td("import.step.metadata", "Metadata")}</span>
          <span className={shared.wizardStepSep}>&rsaquo;</span>
          <span className={`${shared.wizardStep} ${step >= 3 ? "active" : ""} ${step === 3 ? "current" : ""}`}>3. {td("import.step.confirm", "Confirm")}</span>
        </div>
      </div>

      {step === 1 && (
        <div className={shared.formStack}>
          <div className={shared.field}>
            <span>{td("import.cdbFile", "CDB file (required)")}</span>
            <div className={shared.filePickerRow}>
              <input
                readOnly
                value={sourceForm.cdbPath}
                placeholder={td("import.selectCdb", "Select a .cdb file...")}
                title={sourceForm.cdbPath || undefined}
              />
              <button type="button" className={shared.ghostButton} onClick={() => void handleSelectCdb()}>
                {t("action.browse")}
              </button>
            </div>
          </div>

          <div className={shared.field}>
            <span>{td("import.sourceLanguage", "Source language (required)")}</span>
            <TextLanguagePicker
              catalog={config.text_language_catalog}
              value={sourceForm.sourceLanguage}
              onChange={(sourceLanguage) => setSourceForm({ ...sourceForm, sourceLanguage })}
            />
          </div>

          <div className={styles.importSourceDivider}>{td("import.optionalResourcePaths", "Optional resource paths")}</div>

          <FilePickerField
            label="pics/ directory"
            value={sourceForm.picsDir}
            onBrowse={() => void handleBrowseDir("picsDir")}
            onClear={() => setSourceForm({ ...sourceForm, picsDir: "" })}
            placeholder={td("import.cardImagesDir", "Card images directory")}
          />
          <FilePickerField
            label="pics/field/ directory"
            value={sourceForm.fieldPicsDir}
            onBrowse={() => void handleBrowseDir("fieldPicsDir")}
            onClear={() => setSourceForm({ ...sourceForm, fieldPicsDir: "" })}
            placeholder={td("import.fieldImagesDir", "Field images directory")}
          />
          <FilePickerField
            label="script/ directory"
            value={sourceForm.scriptDir}
            onBrowse={() => void handleBrowseDir("scriptDir")}
            onClear={() => setSourceForm({ ...sourceForm, scriptDir: "" })}
            placeholder={td("import.luaScriptsDir", "Lua scripts directory")}
          />
          <FilePickerField
            label="strings.conf"
            value={sourceForm.stringsConfPath}
            onBrowse={() => void handleBrowseFile("stringsConfPath")}
            onClear={() => setSourceForm({ ...sourceForm, stringsConfPath: "" })}
            placeholder={td("import.stringsConfFile", "strings.conf file")}
          />

          <div className={shared.formActions}>
            <button
              type="button"
              className={shared.primaryButton}
              disabled={!canGoNext}
              onClick={handleGoToStep2}
            >
              {td("import.next", "Next")}
            </button>
          </div>
        </div>
      )}

      {step === 2 && (
        <div className={shared.formStack}>
          <div className={shared.field}>
            <span>{td("import.packNameRequired", "Pack name (required)")}</span>
            <input
              value={metadataForm.name}
              onChange={(e) => setMetadataForm({ ...metadataForm, name: e.target.value })}
              placeholder="My Custom Pack"
            />
          </div>

          <div className={shared.packFormRow}>
            <div className={shared.field}>
              <span>{td("import.authorRequired", "Author (required)")}</span>
              <input
                value={metadataForm.author}
                onChange={(e) => setMetadataForm({ ...metadataForm, author: e.target.value })}
                placeholder="Author Name"
              />
            </div>
            <div className={shared.field}>
              <span>{td("import.versionRequired", "Version (required)")}</span>
              <input
                value={metadataForm.version}
                onChange={(e) => setMetadataForm({ ...metadataForm, version: e.target.value })}
                placeholder="1.0.0"
              />
            </div>
          </div>

          <div className={shared.field}>
            <span>{td("pack.form.description", "Description")}</span>
            <textarea
              rows={2}
              value={metadataForm.description}
              onChange={(e) => setMetadataForm({ ...metadataForm, description: e.target.value })}
              placeholder={td("import.optionalDescription", "Optional description")}
            />
          </div>

          <div className={`${shared.packFormRow} ${shared.packFormRowLanguage}`}>
            <div className={shared.field}>
              <span>{td("pack.form.displayLanguages", "Display languages")}</span>
              <LanguageOrderEditor
                catalog={config.text_language_catalog}
                value={metadataForm.displayLanguageOrder}
                onChange={(displayLanguageOrder) => {
                  const defaultExportLanguage = displayLanguageOrder.includes(metadataForm.defaultExportLanguage)
                    ? metadataForm.defaultExportLanguage
                    : displayLanguageOrder[0] ?? "";
                  setMetadataForm({ ...metadataForm, displayLanguageOrder, defaultExportLanguage });
                }}
              />
            </div>
            <div className={shared.field}>
              <span>{td("pack.form.defaultExportLanguage", "Default export language")}</span>
              <TextLanguagePicker
                catalog={config.text_language_catalog}
                value={metadataForm.defaultExportLanguage}
                existingLanguages={metadataForm.displayLanguageOrder}
                onChange={(defaultExportLanguage) => setMetadataForm({ ...metadataForm, defaultExportLanguage })}
              />
            </div>
          </div>

          {previewError && (
            <div className={shared.importErrorBanner}>{previewError}</div>
          )}

          <div className={shared.formActions}>
            <button type="button" className={shared.ghostButton} onClick={() => setStep(1)}>
              {td("import.back", "Back")}
            </button>
            <button
              type="button"
              className={shared.primaryButton}
              disabled={busy !== null}
              onClick={() => void handlePreview()}
            >
              {busy === "preview" ? td("import.previewing", "Previewing...") : td("import.preview", "Preview Import")}
            </button>
          </div>
        </div>
      )}

      {step === 3 && previewResult && (
        <div className={shared.importPreviewStep}>
          <div className={shared.importPreviewSummary}>
            <div className={shared.importStat}>
              <span className={shared.importStatValue}>{previewResult.data.card_count}</span>
              <span className={shared.importStatLabel}>{td("pack.tabs.cards", "Cards")}</span>
            </div>
            <div className={shared.importStat}>
              <span
                className={shared.importStatValue}
                data-level={previewResult.data.error_count > 0 ? "error" : undefined}
              >
                {previewResult.data.error_count}
              </span>
              <span className={shared.importStatLabel}>{td("import.errors", "Errors")}</span>
            </div>
            <div className={shared.importStat}>
              <span
                className={shared.importStatValue}
                data-level={previewResult.data.warning_count > 0 ? "warning" : undefined}
              >
                {previewResult.data.warning_count}
              </span>
              <span className={shared.importStatLabel}>{td("import.warnings", "Warnings")}</span>
            </div>
            <div className={shared.importStat}>
              <span className={shared.importStatValue}>
                {previewResult.data.missing_main_image_count}
              </span>
              <span className={shared.importStatLabel}>{td("import.missingImages", "Missing Images")}</span>
            </div>
            <div className={shared.importStat}>
              <span className={shared.importStatValue}>
                {previewResult.data.missing_script_count}
              </span>
              <span className={shared.importStatLabel}>{td("import.missingScripts", "Missing Scripts")}</span>
            </div>
            <div className={shared.importStat}>
              <span className={shared.importStatValue}>
                {previewResult.data.missing_field_image_count}
              </span>
              <span className={shared.importStatLabel}>{td("import.missingFieldImages", "Missing Field Imgs")}</span>
            </div>
          </div>

          {previewResult.data.error_count > 0 && (
            <div className={shared.importErrorBanner}>
              {td("import.blockingErrors", "Import has {count} blocking error{plural}. Fix the source and try again.", {
                count: previewResult.data.error_count,
                plural: previewResult.data.error_count > 1 ? "s" : "",
              })}
            </div>
          )}

          {previewResult.data.issues.length > 0 && (
            <div className={shared.importIssuesList}>
              <strong className={shared.importIssuesHeader}>
                {td("import.issuesCount", "Issues ({count})", { count: previewResult.data.issues.length })}
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
              {td("import.completed", "Import completed successfully.")}
            </div>
          )}

          <div className={shared.formActions}>
            {!jobDone && !importing && (
              <>
                <button type="button" className={shared.ghostButton} onClick={handleBackFromStep3}>
                  {td("import.back", "Back")}
                </button>
                <button
                  type="button"
                  className={shared.primaryButton}
                  disabled={previewResult.data.error_count > 0 || busy !== null}
                  onClick={() => void handleExecute()}
                >
                  {busy === "execute" ? td("import.submitting", "Submitting...") : t("action.import")}
                </button>
              </>
            )}

            {importing && (
              <button type="button" className={shared.ghostButton} disabled>
                {td("import.importing", "Importing...")}
              </button>
            )}

            {jobSucceeded && (
              <button
                type="button"
                className={shared.primaryButton}
                disabled={openingPack}
                onClick={() => void handleOpenImportedPack()}
              >
                {openingPack ? td("pack.opening", "Opening...") : td("import.openImportedPack", "Open Imported Pack")}
              </button>
            )}

            {jobFailed && (
              <>
                <button type="button" className={shared.ghostButton} onClick={handleBackFromStep3}>
                  {td("import.back", "Back")}
                </button>
                <span className={shared.importFailHint}>{td("import.jobFailed", "Import job failed. Check errors above and try again.")}</span>
              </>
            )}
          </div>
        </div>
      )}
    </section>
  );
}

function FilePickerField({
  label,
  value,
  onBrowse,
  onClear,
  placeholder,
}: {
  label: string;
  value: string;
  onBrowse: () => void;
  onClear: () => void;
  placeholder: string;
}) {
  const { t } = useAppI18n();

  return (
    <div className={shared.field}>
      <span>{label}</span>
      <div className={shared.filePickerRow}>
        <input readOnly value={value} placeholder={placeholder} title={value || undefined} />
        <button type="button" className={shared.ghostButton} onClick={onBrowse}>
          {t("action.browse")}
        </button>
        {value && (
          <button type="button" className={shared.ghostButton} onClick={onClear}>
            {t("action.clear")}
          </button>
        )}
      </div>
    </div>
  );
}
