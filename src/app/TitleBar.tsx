import { useAppI18n } from "../shared/i18n";
import styles from "./TitleBar.module.css";

type WindowAction = "minimize" | "toggle-maximize" | "close";

interface TitleBarProps {
  workspaceName: string;
  maximized: boolean;
  onWindowAction: (action: WindowAction) => void;
}

export function TitleBar({ workspaceName, maximized, onWindowAction }: TitleBarProps) {
  const { t } = useAppI18n();
  const appIconSrc = `${import.meta.env.BASE_URL}app-icon.png`;

  return (
    <header
      className={styles.titlebar}
      data-tauri-drag-region
      onDoubleClick={() => void onWindowAction("toggle-maximize")}
    >
      <div className={styles.titlebarLeft} data-tauri-drag-region>
        <img className={styles.appIcon} src={appIconSrc} alt="YGOCMG" draggable={false} />
        <span className={styles.titlebarName}>{workspaceName}</span>
      </div>
      <div className={styles.titlebarSpacer} data-tauri-drag-region />
      <div className={styles.windowControls}>
        <button type="button" className={styles.winBtn} aria-label={t("window.minimize")} onClick={() => void onWindowAction("minimize")}>
          <svg width="10" height="1" viewBox="0 0 10 1">
            <rect width="10" height="1" fill="currentColor" />
          </svg>
        </button>
        <button type="button" className={styles.winBtn} aria-label={maximized ? t("window.restore") : t("window.maximize")} onClick={() => void onWindowAction("toggle-maximize")}>
          {maximized ? (
            <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" strokeWidth="1">
              <rect x="2" y="0" width="8" height="8" rx="0.5" />
              <rect x="0" y="2" width="8" height="8" rx="0.5" fill="var(--panel)" />
            </svg>
          ) : (
            <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" strokeWidth="1">
              <rect x="0.5" y="0.5" width="9" height="9" rx="0.5" />
            </svg>
          )}
        </button>
        <button type="button" className={`${styles.winBtn} ${styles.winClose}`} aria-label={t("window.close")} onClick={() => void onWindowAction("close")}>
          <svg width="10" height="10" viewBox="0 0 10 10" stroke="currentColor" strokeWidth="1.2">
            <line x1="0" y1="0" x2="10" y2="10" />
            <line x1="10" y1="0" x2="0" y2="10" />
          </svg>
        </button>
      </div>
    </header>
  );
}
