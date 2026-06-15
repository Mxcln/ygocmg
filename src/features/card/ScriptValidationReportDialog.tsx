import type { LuaValidationIssue, LuaValidationReport } from "../../shared/contracts/script";
import { useAppI18n, type AppMessageId } from "../../shared/i18n";
import shared from "../../shared/styles/shared.module.css";
import styles from "./ScriptValidationReportDialog.module.css";

type Translate = (id: string, values?: Record<string, string | number>) => string;

interface ScriptValidationReportDialogProps {
  report: LuaValidationReport | null;
  open: boolean;
  onClose: () => void;
}

function issueLocation(issue: LuaValidationIssue, t: Translate): string | null {
  const parts: string[] = [];
  if (issue.line !== undefined) {
    parts.push(t("card.scriptValidation.line", { line: issue.line }));
  }
  if (issue.column !== undefined) {
    parts.push(t("card.scriptValidation.column", { column: issue.column }));
  }
  return parts.length ? parts.join(" ") : null;
}

export function ScriptValidationReportContent({
  report,
  t,
}: {
  report: LuaValidationReport;
  t: Translate;
}) {
  return (
    <div className={styles.reportContent}>
      <section className={styles.summaryBlock} data-status={report.status}>
        <div className={styles.summaryMeta}>
          <div>
            <span className={styles.kicker}>{t("card.scriptValidation.status")}</span>
            <strong>{report.status}</strong>
          </div>
          <div>
            <span className={styles.kicker}>{t("card.scriptValidation.confidence")}</span>
            <strong>{report.confidence}</strong>
          </div>
        </div>
        <p>{report.summary}</p>
      </section>

      <section className={styles.section}>
        <h3>{t("card.scriptValidation.stages")}</h3>
        <div className={styles.stageList}>
          {report.stages.map((stage) => (
            <div className={styles.stageRow} data-status={stage.status} key={stage.stage}>
              <strong>{stage.stage}</strong>
              <span>{stage.status}</span>
              <span>{t("card.scriptValidation.durationMs", { duration: stage.durationMs })}</span>
              <span>{t("card.scriptValidation.issueCount", { count: stage.issues.length })}</span>
            </div>
          ))}
        </div>
      </section>

      <section className={styles.section}>
        <h3>{t("card.scriptValidation.issues")}</h3>
        {report.issues.length ? (
          <ul className={styles.issueList}>
            {report.issues.map((issue, index) => {
              const location = issueLocation(issue, t);
              return (
                <li
                  className={styles.issueItem}
                  data-severity={issue.severity}
                  key={`${issue.code}-${index}`}
                >
                  <div className={styles.issueHead}>
                    <strong>{issue.code}</strong>
                    <span>{issue.severity}</span>
                    <span>{issue.stage}</span>
                    {location && <span>{location}</span>}
                  </div>
                  <p>{issue.message}</p>
                  {issue.suggestion && (
                    <p className={styles.suggestion}>
                      {t("card.scriptValidation.suggestion")}: {issue.suggestion}
                    </p>
                  )}
                </li>
              );
            })}
          </ul>
        ) : (
          <p className={styles.emptyText}>{t("card.scriptValidation.noIssues")}</p>
        )}
      </section>

      <section className={styles.section}>
        <h3>{t("card.scriptValidation.limitations")}</h3>
        {report.limitations.length ? (
          <ul className={styles.limitList}>
            {report.limitations.map((limitation, index) => (
              <li key={`${limitation}-${index}`}>{limitation}</li>
            ))}
          </ul>
        ) : (
          <p className={styles.emptyText}>{t("card.scriptValidation.noLimitations")}</p>
        )}
      </section>
    </div>
  );
}

export function ScriptValidationReportDialog({
  report,
  open,
  onClose,
}: ScriptValidationReportDialogProps) {
  const { t } = useAppI18n();
  if (!open || !report) return null;
  const translate: Translate = (id, values) => t(id as AppMessageId, values);

  return (
    <div className={styles.dialogLayer}>
      <div className={styles.dialogBackdrop} onClick={onClose} />
      <section
        className={styles.dialogBox}
        role="dialog"
        aria-modal="true"
        aria-labelledby="script-validation-report-title"
      >
        <header className={shared.modalHeader}>
          <h2 id="script-validation-report-title">{t("card.scriptValidation.title")}</h2>
          <button type="button" className={shared.modalCloseButton} onClick={onClose}>
            {t("action.close")}
          </button>
        </header>
        <div className={shared.modalBody}>
          <ScriptValidationReportContent report={report} t={translate} />
        </div>
      </section>
    </div>
  );
}
