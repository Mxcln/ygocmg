import type { PointerEvent as ReactPointerEvent } from "react";
import { useShellStore } from "../shared/stores/shellStore";
import type { ModalType } from "../shared/stores/shellStore";
import { useAppI18n } from "../shared/i18n";
import styles from "./AppSidebar.module.css";

interface AppSidebarProps {
  hasWorkspace: boolean;
  isStandardView: boolean;
  onPackClick: (packId: string) => void;
  onClosePack: (packId: string) => void;
  onOpenStandardPack: () => void;
  onBeginResize: (event: ReactPointerEvent<HTMLDivElement>) => void;
}

export function AppSidebar({
  hasWorkspace,
  isStandardView,
  onPackClick,
  onClosePack,
  onOpenStandardPack,
  onBeginResize,
}: AppSidebarProps) {
  const { t } = useAppI18n();
  const modal = useShellStore((s) => s.modal);
  const openModal = useShellStore((s) => s.openModal);
  const openPackIds = useShellStore((s) => s.openPackIds);
  const activeView = useShellStore((s) => s.activeView);
  const packMetadataMap = useShellStore((s) => s.packMetadataMap);

  function actionBtnClass(type: ModalType) {
    return `${styles.actionBtn} ${modal?.type === type ? "active" : ""}`;
  }

  return (
    <aside className={styles.sidebar}>
      <div className={styles.sidebarActions}>
        <button type="button" className={actionBtnClass("workspace")} title={t("sidebar.workspace")} onClick={() => openModal("workspace")}>
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3">
            <rect x="1" y="2" width="14" height="12" rx="1.5" />
            <line x1="1" y1="6" x2="15" y2="6" />
          </svg>
        </button>
        <button type="button" className={actionBtnClass("export")} title={t("sidebar.export")} disabled={!hasWorkspace} onClick={() => openModal("export")}>
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3">
            <path d="M2 11v3h12v-3" />
            <path d="M8 2v8" />
            <path d="M5 7l3 3 3-3" />
          </svg>
        </button>
        <button type="button" className={actionBtnClass("settings")} title={t("sidebar.settings")} onClick={() => openModal("settings")}>
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3">
            <circle cx="8" cy="8" r="2.5" />
            <path d="M8 1v2m0 10v2M1 8h2m10 0h2M2.9 2.9l1.5 1.5m7.2 7.2l1.5 1.5M13.1 2.9l-1.5 1.5M4.4 11.6l-1.5 1.5" />
          </svg>
        </button>
      </div>

      <div className={styles.packList}>
        {openPackIds.map((packId) => {
          const meta = packMetadataMap[packId];
          const isActive = activeView?.type === "custom_pack" && activeView.packId === packId;
          return (
            <div key={packId} className={`${styles.packItemRow} ${isActive ? "active" : ""}`}>
              <button
                type="button"
                className={styles.packItemName}
                onClick={() => onPackClick(packId)}
                title={meta?.name ?? packId}
              >
                {meta?.name ?? packId}
              </button>
              <button
                type="button"
                className={styles.packCloseBtn}
                title={t("sidebar.closePack")}
                onClick={(event) => {
                  event.stopPropagation();
                  onClosePack(packId);
                }}
              >
                <svg width="8" height="8" viewBox="0 0 8 8" stroke="currentColor" strokeWidth="1.2">
                  <line x1="0" y1="0" x2="8" y2="8" />
                  <line x1="8" y1="0" x2="0" y2="8" />
                </svg>
              </button>
            </div>
          );
        })}

        <button
          type="button"
          className={`${styles.packItem} ${styles.packAdd}`}
          onClick={() => openModal("addPack")}
          disabled={!hasWorkspace}
          title={hasWorkspace ? t("sidebar.openOrCreatePack") : t("sidebar.openWorkspaceFirst")}
        >
          +
        </button>
      </div>

      <div className={styles.sidebarBottom}>
        <button
          type="button"
          className={`${styles.packItem} ${styles.packStandard} ${isStandardView ? "active" : ""}`}
          onClick={onOpenStandardPack}
        >
          {t("sidebar.standardPack")}
        </button>
      </div>

      <div
        className={styles.sidebarResizeHandle}
        role="separator"
        aria-orientation="vertical"
        aria-label={t("sidebar.resize")}
        onPointerDown={onBeginResize}
      />
    </aside>
  );
}
