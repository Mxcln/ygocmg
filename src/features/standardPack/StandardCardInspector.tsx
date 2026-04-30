import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { convertFileSrc } from "@tauri-apps/api/core";
import { standardPackApi } from "../../shared/api/standardPackApi";
import { configApi } from "../../shared/api/configApi";
import type { CardEntity } from "../../shared/contracts/card";
import { CardInfoForm } from "../card/CardInfoForm";
import { CardTextForm } from "../card/CardTextForm";
import { useAppI18n } from "../../shared/i18n";
import drawerStyles from "../card/CardEditDrawer.module.css";
import assetStyles from "../card/CardAssetBar.module.css";
import shared from "../../shared/styles/shared.module.css";
import styles from "./StandardCardInspector.module.css";

interface StandardCardInspectorProps {
  code: number;
  onClose: () => void;
}

type InspectorTab = "text" | "info";

function textLanguages(card: CardEntity, available: string[]): string[] {
  return available.length > 0 ? available : Object.keys(card.texts);
}

export function StandardCardInspector({ code, onClose }: StandardCardInspectorProps) {
  const { t } = useAppI18n();
  const [activeTab, setActiveTab] = useState<InspectorTab>("text");

  const { data: detail, isLoading, error } = useQuery({
    queryKey: ["standard-card", code],
    queryFn: () => standardPackApi.getCard({ code }),
  });
  const { data: config } = useQuery({
    queryKey: ["config-for-standard-card"],
    queryFn: () => configApi.loadConfig(),
  });

  const card = detail?.card ?? null;
  const languages = card ? textLanguages(card, detail?.available_languages ?? []) : [];
  const titleLanguage = languages[0] ?? "";
  const imageSrc = detail?.asset_state.has_image && detail.ygopro_path
    ? `${convertFileSrc(`${detail.ygopro_path}/pics/${code}.jpg`)}?standard=${code}`
    : null;

  return (
    <>
      <div className={drawerStyles.cardEditBackdrop} onClick={onClose} />
      <div className={drawerStyles.cardEditDrawer}>
        <div className={drawerStyles.cardEditHeader}>
          <div className={drawerStyles.cardEditHeaderLeft}>
            <button type="button" className={shared.ghostButton} onClick={onClose}>
              {t("action.close")}
            </button>
            <div className={styles.inspectorTitle}>
              <strong>{card?.texts[titleLanguage]?.name || t("card.titleWithCode", { code })}</strong>
              <span>{code}</span>
            </div>
          </div>
          <div className={drawerStyles.cardEditHeaderSpacer} />
          <span className={styles.readonlyChip}>{t("standard.card.readOnly")}</span>
        </div>

        {error && (
          <div className={drawerStyles.cardEditError}>{t("standard.card.failed")}</div>
        )}

        {isLoading && !card ? (
          <div className={shared.cardListEmpty}>
            <p>{t("card.loading")}</p>
          </div>
        ) : card && detail ? (
          <div className={drawerStyles.cardEditBody}>
            <div className={assetStyles.cardAssetBar}>
              <div className={assetStyles.cardPicPreview}>
                {imageSrc ? <img src={imageSrc} alt={t("card.asset.cardImageAlt")} /> : t("card.asset.noImage")}
              </div>
              <div className={styles.assetReadonlyGrid}>
                <span>{t("card.asset.image")}</span>
                <strong>{detail.asset_state.has_image ? t("common.present") : t("common.missing")}</strong>
                <span>{t("card.asset.script")}</span>
                <strong>{detail.asset_state.has_script ? t("common.present") : t("common.missing")}</strong>
                <span>{t("card.asset.field")}</span>
                <strong>{detail.asset_state.has_field_image ? t("common.present") : t("common.missing")}</strong>
              </div>
            </div>

            <div className={drawerStyles.cardFormArea}>
              <div className={drawerStyles.cardFormTabs}>
                <button
                  type="button"
                  className={`${drawerStyles.cardFormTab} ${activeTab === "text" ? "active" : ""}`}
                  onClick={() => setActiveTab("text")}
                >
                  {t("card.tab.text")}
                </button>
                <button
                  type="button"
                  className={`${drawerStyles.cardFormTab} ${activeTab === "info" ? "active" : ""}`}
                  onClick={() => setActiveTab("info")}
                >
                  {t("card.tab.info")}
                </button>
              </div>

              <div className={drawerStyles.cardFormContent}>
                {activeTab === "text" ? (
                  <CardTextForm
                    draft={card}
                    catalog={config?.text_language_catalog ?? []}
                    displayLanguageOrder={languages}
                    readonly
                  />
                ) : (
                  <CardInfoForm draft={card} readonly />
                )}
              </div>
            </div>
          </div>
        ) : null}
      </div>
    </>
  );
}
