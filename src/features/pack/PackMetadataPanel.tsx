import { useState } from "react";
import type { ReactNode } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useShellStore } from "../../shared/stores/shellStore";
import { packApi } from "../../shared/api/packApi";
import type { GlobalConfig } from "../../shared/contracts/config";
import { formatError, formatTimestamp } from "../../shared/utils/format";
import { compactLanguageLabel, languageLabel } from "../../shared/utils/language";
import { useAppI18n } from "../../shared/i18n";
import { LanguageOrderEditor } from "../language/LanguageOrderEditor";
import { TextLanguagePicker } from "../language/TextLanguagePicker";
import type { NoticeTone } from "../../app/NoticeBanner";
import shared from "../../shared/styles/shared.module.css";
import styles from "./PackMetadataPanel.module.css";

interface PackMetadataPanelProps {
  config: GlobalConfig;
  onNotice: (tone: NoticeTone, title: string, detail: string) => void;
  onPackDeleted: (packId: string) => void;
  children: ReactNode;
}

export function PackMetadataPanel({
  config,
  onNotice,
  onPackDeleted,
  children,
}: PackMetadataPanelProps) {
  const { t } = useAppI18n();
  const activePackId = useShellStore((s) => s.activePackId);
  const activeView = useShellStore((s) => s.activeView);
  const packMetadataMap = useShellStore((s) => s.packMetadataMap);
  const updatePackMetadata = useShellStore((s) => s.updatePackMetadata);
  const setPackOverviews = useShellStore((s) => s.setPackOverviews);
  const openDialog = useShellStore((s) => s.openDialog);
  const closeDialog = useShellStore((s) => s.closeDialog);
  const queryClient = useQueryClient();

  const [metaExpanded, setMetaExpanded] = useState(false);
  const [metaEditing, setMetaEditing] = useState(false);
  const [metaDraft, setMetaDraft] = useState<{
    name: string;
    packCode: string;
    author: string;
    version: string;
    description: string;
    displayLanguageOrder: string[];
    defaultExportLanguage: string;
  } | null>(null);
  const [metaSaving, setMetaSaving] = useState(false);

  const packId = activeView?.type === "custom_pack" ? activeView.packId : activePackId;
  const metadata = packId ? packMetadataMap[packId] : null;

  const preferredTextLanguages = metadata
    ? metadata.display_language_order
        .map((lang) => compactLanguageLabel(config.text_language_catalog, lang))
        .join(", ")
    : t("common.none");
  const summaryDetail = metadata
    ? `${metadata.author} · v${metadata.version} · ${preferredTextLanguages}`
    : t("pack.metadata.loading");

  function handleStartEdit() {
    if (!metadata) return;
    setMetaDraft({
      name: metadata.name,
      packCode: metadata.pack_code ?? "",
      author: metadata.author,
      version: metadata.version,
      description: metadata.description || "",
      displayLanguageOrder: metadata.display_language_order,
      defaultExportLanguage: metadata.default_export_language || "",
    });
    setMetaEditing(true);
  }

  function handleCancelEdit() {
    setMetaEditing(false);
    setMetaDraft(null);
  }

  async function handleSave() {
    if (!packId || !metaDraft) return;
    const trimmedName = metaDraft.name.trim();
    if (!trimmedName) {
      onNotice("error", t("pack.metadata.validationError"), t("pack.metadata.nameEmpty"));
      return;
    }

    setMetaSaving(true);
    try {
      const updated = await packApi.updatePackMetadata({
        packId,
        name: trimmedName,
        packCode: metaDraft.packCode.trim() || null,
        author: metaDraft.author.trim(),
        version: metaDraft.version.trim(),
        description: metaDraft.description.trim() || null,
        displayLanguageOrder: metaDraft.displayLanguageOrder,
        defaultExportLanguage: metaDraft.defaultExportLanguage.trim() || null,
      });

      updatePackMetadata(packId, updated);
      void queryClient.invalidateQueries({ queryKey: ["cards", packId] });
      void queryClient.invalidateQueries({ queryKey: ["strings", packId] });
      const overviews = await packApi.listPackOverviews();
      setPackOverviews(overviews);
      setMetaEditing(false);
      setMetaDraft(null);
      onNotice("success", t("pack.metadata.saved.title"), t("pack.metadata.saved.detail"));
    } catch (err) {
      onNotice("error", t("pack.metadata.saveFailed"), formatError(err));
    } finally {
      setMetaSaving(false);
    }
  }

  function handleRequestDelete() {
    if (!packId || !metadata) return;
    const deletePackId = packId;
    openDialog({
      kind: "confirm",
      title: t("pack.delete.title"),
      message: t("pack.delete.message", { name: metadata.name }),
      confirmLabel: t("pack.delete.confirm"),
      cancelLabel: t("action.cancel"),
      danger: true,
      onConfirm: async () => {
        try {
          await packApi.deletePack({ packId: deletePackId });
          closeDialog();
          onPackDeleted(deletePackId);
        } catch (err) {
          onNotice("error", t("pack.delete.failed"), formatError(err));
        }
      },
    });
  }

  return (
    <>
      <div className={styles.metaBar}>
        <div className={styles.metaSummary}>
          <strong className={styles.metaPackName} title={metadata?.name ?? packId ?? ""}>
            {metadata?.name ?? packId}
          </strong>
          {metadata?.pack_code && (
            <span className={styles.metaPackCode} title={metadata.pack_code}>
              {metadata.pack_code}
            </span>
          )}
          <span className={styles.metaDetail} title={summaryDetail}>
            {summaryDetail}
          </span>
        </div>
        <button
          type="button"
          className={styles.metaToggle}
          onClick={() => setMetaExpanded(!metaExpanded)}
          aria-label={metaExpanded ? t("pack.metadata.collapse") : t("pack.metadata.expand")}
        >
          <svg
            width="12"
            height="12"
            viewBox="0 0 12 12"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            style={{ transform: metaExpanded ? "rotate(180deg)" : "none", transition: "transform 150ms" }}
          >
            <path d="M2 4l4 4 4-4" />
          </svg>
        </button>
      </div>

      <div className={styles.workAreaContent}>
        {metaExpanded && metadata && (
          <>
            <div
              className={styles.drawerBackdrop}
              onClick={() => {
                if (!metaEditing) setMetaExpanded(false);
              }}
            />
            <div className={styles.expanded}>
              {metaEditing && metaDraft ? (
                <>
                  <div className={styles.metaGrid}>
                    <div className={styles.metaField}>
                      <span className={styles.metaFieldLabel}>{t("pack.metadata.name")}</span>
                      <input
                        className={styles.metaEditInput}
                        value={metaDraft.name}
                        onChange={(e) => setMetaDraft({ ...metaDraft, name: e.target.value })}
                      />
                    </div>
                    <div className={styles.metaField}>
                      <span className={styles.metaFieldLabel}>{t("pack.metadata.packCode")}</span>
                      <input
                        className={styles.metaEditInput}
                        value={metaDraft.packCode}
                        onChange={(e) => setMetaDraft({ ...metaDraft, packCode: e.target.value })}
                        placeholder="BZDM"
                      />
                    </div>
                    <div className={styles.metaField}>
                      <span className={styles.metaFieldLabel}>{t("pack.metadata.author")}</span>
                      <input
                        className={styles.metaEditInput}
                        value={metaDraft.author}
                        onChange={(e) => setMetaDraft({ ...metaDraft, author: e.target.value })}
                      />
                    </div>
                    <div className={styles.metaField}>
                      <span className={styles.metaFieldLabel}>{t("pack.metadata.version")}</span>
                      <input
                        className={styles.metaEditInput}
                        value={metaDraft.version}
                        onChange={(e) => setMetaDraft({ ...metaDraft, version: e.target.value })}
                      />
                    </div>
                    <div className={styles.metaField}>
                      <span className={styles.metaFieldLabel}>{t("pack.metadata.preferredTextLanguages")}</span>
                      <LanguageOrderEditor
                        catalog={config.text_language_catalog}
                        value={metaDraft.displayLanguageOrder}
                        existingLanguages={metadata.display_language_order}
                        onChange={(displayLanguageOrder) => {
                          const defaultExportLanguage = displayLanguageOrder.includes(metaDraft.defaultExportLanguage)
                            ? metaDraft.defaultExportLanguage
                            : displayLanguageOrder[0] ?? "";
                          setMetaDraft({ ...metaDraft, displayLanguageOrder, defaultExportLanguage });
                        }}
                      />
                    </div>
                    <div className={styles.metaField}>
                      <span className={styles.metaFieldLabel}>{t("pack.metadata.defaultExportLanguage")}</span>
                      <TextLanguagePicker
                        catalog={config.text_language_catalog}
                        value={metaDraft.defaultExportLanguage}
                        existingLanguages={metadata.display_language_order}
                        onChange={(defaultExportLanguage) => setMetaDraft({ ...metaDraft, defaultExportLanguage })}
                      />
                    </div>
                    <div className={styles.metaField}>
                      <span className={styles.metaFieldLabel}>{t("pack.metadata.created")}</span>
                      <span className={styles.metaFieldValue}>{formatTimestamp(metadata.created_at)}</span>
                    </div>
                    <div className={styles.metaField}>
                      <span className={styles.metaFieldLabel}>{t("pack.metadata.updated")}</span>
                      <span className={styles.metaFieldValue}>{formatTimestamp(metadata.updated_at)}</span>
                    </div>
                    <div className={`${styles.metaField} ${styles.metaFieldWide}`}>
                      <span className={styles.metaFieldLabel}>{t("pack.metadata.description")}</span>
                      <textarea
                        className={styles.metaEditInput}
                        value={metaDraft.description}
                        onChange={(e) => setMetaDraft({ ...metaDraft, description: e.target.value })}
                        rows={3}
                      />
                    </div>
                  </div>
                  <div className={styles.metaActions}>
                    <button type="button" className={shared.primaryButton} onClick={() => void handleSave()} disabled={metaSaving}>
                      {metaSaving ? t("pack.metadata.saving") : t("action.save")}
                    </button>
                    <button type="button" className={shared.ghostButton} onClick={handleCancelEdit} disabled={metaSaving}>
                      {t("action.cancel")}
                    </button>
                  </div>
                </>
              ) : (
                <>
                  <div className={styles.metaGrid}>
                    <div className={styles.metaField}>
                      <span className={styles.metaFieldLabel}>{t("pack.metadata.name")}</span>
                      <span className={`${styles.metaFieldValue} ${styles.metaFieldValueInline}`} title={metadata.name}>
                        {metadata.name}
                      </span>
                    </div>
                    <div className={styles.metaField}>
                      <span className={styles.metaFieldLabel}>{t("pack.metadata.packCode")}</span>
                      <span className={`${styles.metaFieldValue} ${styles.metaFieldValueInline}`} title={metadata.pack_code || t("common.none")}>
                        {metadata.pack_code || t("common.none")}
                      </span>
                    </div>
                    <div className={styles.metaField}>
                      <span className={styles.metaFieldLabel}>{t("pack.metadata.author")}</span>
                      <span className={`${styles.metaFieldValue} ${styles.metaFieldValueInline}`} title={metadata.author}>
                        {metadata.author}
                      </span>
                    </div>
                    <div className={styles.metaField}>
                      <span className={styles.metaFieldLabel}>{t("pack.metadata.version")}</span>
                      <span className={`${styles.metaFieldValue} ${styles.metaFieldValueInline}`} title={metadata.version}>
                        {metadata.version}
                      </span>
                    </div>
                    <div className={styles.metaField}>
                      <span className={styles.metaFieldLabel}>{t("pack.metadata.preferredTextLanguages")}</span>
                      <span className={styles.metaFieldValue} title={preferredTextLanguages}>
                        {preferredTextLanguages}
                      </span>
                    </div>
                    <div className={styles.metaField}>
                      <span className={styles.metaFieldLabel}>{t("pack.metadata.defaultExportLanguage")}</span>
                      <span className={`${styles.metaFieldValue} ${styles.metaFieldValueInline}`} title={metadata.default_export_language || t("common.none")}>
                        {metadata.default_export_language
                          ? languageLabel(config.text_language_catalog, metadata.default_export_language)
                          : t("common.none")}
                      </span>
                    </div>
                    <div className={styles.metaField}>
                      <span className={styles.metaFieldLabel}>{t("pack.metadata.created")}</span>
                      <span className={styles.metaFieldValue}>{formatTimestamp(metadata.created_at)}</span>
                    </div>
                    <div className={styles.metaField}>
                      <span className={styles.metaFieldLabel}>{t("pack.metadata.updated")}</span>
                      <span className={styles.metaFieldValue}>{formatTimestamp(metadata.updated_at)}</span>
                    </div>
                    <div className={`${styles.metaField} ${styles.metaFieldWide}`}>
                      <span className={styles.metaFieldLabel}>{t("pack.metadata.description")}</span>
                      <span className={`${styles.metaFieldValue} ${styles.metaFieldValueDescription}`} title={metadata.description || t("common.none")}>
                        {metadata.description || t("common.none")}
                      </span>
                    </div>
                  </div>
                  <div className={styles.metaActions}>
                    <button type="button" className={shared.ghostButton} onClick={handleStartEdit}>
                      {t("pack.metadata.edit")}
                    </button>
                    <button type="button" className={shared.dangerButton} onClick={handleRequestDelete}>
                      {t("pack.delete.confirm")}
                    </button>
                  </div>
                </>
              )}
            </div>
          </>
        )}
        {children}
      </div>
    </>
  );
}
