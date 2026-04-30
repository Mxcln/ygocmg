import { useEffect } from "react";
import { useShellStore } from "../../shared/stores/shellStore";
import { formatError, formatValidationIssue } from "../../shared/utils/format";
import { formatIssueDetail } from "../../shared/utils/messages";
import { useAppI18n } from "../../shared/i18n";
import shared from "../../shared/styles/shared.module.css";
import styles from "./AppDialog.module.css";

export function AppDialog() {
  const { t } = useAppI18n();
  const dialogState = useShellStore((s) => s.dialog);
  const busy = useShellStore((s) => s.dialogBusy);
  const closeDialog = useShellStore((s) => s.closeDialog);
  const setDialogBusy = useShellStore((s) => s.setDialogBusy);
  const setDialogError = useShellStore((s) => s.setDialogError);

  if (!dialogState) return null;
  const dialog = dialogState;

  async function handleConfirm() {
    try {
      setDialogBusy(true);
      setDialogError(null);
      await dialog.onConfirm();
    } catch (error) {
      setDialogError(formatError(error));
    } finally {
      setDialogBusy(false);
    }
  }

  const canClose = !busy;

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        if (canClose) {
          event.preventDefault();
          closeDialog();
        }
        return;
      }

      if (event.key === "Enter") {
        const target = event.target;
        if (
          target instanceof HTMLElement &&
          (target.tagName === "TEXTAREA" ||
            target.tagName === "BUTTON" ||
            target.tagName === "A")
        ) {
          return;
        }
        event.preventDefault();
        void handleConfirm();
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [canClose, closeDialog, dialog]);

  return (
    <div className={styles.dialogLayer}>
      <div
        className={styles.dialogBackdrop}
        onClick={() => {
          if (canClose) closeDialog();
        }}
      />
      <section className={styles.dialogBox} role="dialog" aria-modal="true" aria-labelledby="app-dialog-title">
        <header className={shared.modalHeader}>
          <div>
            <h2 id="app-dialog-title">{dialog.title}</h2>
            <p className={styles.summary}>{dialog.message}</p>
          </div>
        </header>
        {dialog.kind === "warning" && (
          <div className={styles.warningList}>
            <ul>
              {dialog.warnings.map((warning, index) => (
                <li key={`${warning.code}-${index}`}>
                  <strong>{formatValidationIssue(warning)}</strong>
                  {formatIssueDetail(warning) && <span>{formatIssueDetail(warning)}</span>}
                </li>
              ))}
            </ul>
          </div>
        )}
        {dialog.errorMessage && (
          <div className={styles.error} role="alert">
            {dialog.errorMessage}
          </div>
        )}
        <div className={styles.actions}>
          <button
            type="button"
            className={shared.ghostButton}
            onClick={closeDialog}
            disabled={!canClose}
          >
            {dialog.cancelLabel}
          </button>
          <button
            type="button"
            className={dialog.kind === "confirm" && dialog.danger ? shared.dangerButton : shared.primaryButton}
            onClick={() => void handleConfirm()}
            disabled={busy}
          >
            {busy ? t("action.working") : dialog.confirmLabel}
          </button>
        </div>
      </section>
    </div>
  );
}
