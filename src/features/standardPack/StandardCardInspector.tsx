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
  const { t, td } = useAppI18n();
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
              <strong>{card?.texts[titleLanguage]?.name || td("card.titleWithCode", "Card {code}", { code })}</strong>
              <span>{code}</span>
            </div>
          </div>
          <div className={drawerStyles.cardEditHeaderSpacer} />
          <span className={styles.readonlyChip}>{td("standard.card.readOnly", "Read-only")}</span>
        </div>

        {error && (
          <div className={drawerStyles.cardEditError}>{td("standard.card.failed", "Failed to load standard card.")}</div>
        )}

        {isLoading && !card ? (
          <div className={shared.cardListEmpty}>
            <p>{td("card.loading", "Loading card...")}</p>
          </div>
        ) : card && detail ? (
          <div className={drawerStyles.cardEditBody}>
            <div className={assetStyles.cardAssetBar}>
              <div className={assetStyles.cardPicPreview}>
                {imageSrc ? <img src={imageSrc} alt={td("card.asset.cardImageAlt", "Card")} /> : td("card.asset.noImage", "No Image")}
              </div>
              <div className={styles.assetReadonlyGrid}>
                <span>{td("card.asset.image", "Image")}</span>
                <strong>{detail.asset_state.has_image ? td("common.present", "Present") : td("common.missing", "Missing")}</strong>
                <span>{td("card.asset.script", "Script")}</span>
                <strong>{detail.asset_state.has_script ? td("common.present", "Present") : td("common.missing", "Missing")}</strong>
                <span>{td("card.asset.field", "Field")}</span>
                <strong>{detail.asset_state.has_field_image ? td("common.present", "Present") : td("common.missing", "Missing")}</strong>
              </div>
            </div>

            <div className={drawerStyles.cardFormArea}>
              <div className={drawerStyles.cardFormTabs}>
                <button
                  type="button"
                  className={`${drawerStyles.cardFormTab} ${activeTab === "text" ? "active" : ""}`}
                  onClick={() => setActiveTab("text")}
                >
                  {td("card.tab.text", "Text")}
                </button>
                <button
                  type="button"
                  className={`${drawerStyles.cardFormTab} ${activeTab === "info" ? "active" : ""}`}
                  onClick={() => setActiveTab("info")}
                >
                  {td("card.tab.info", "Info")}
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
