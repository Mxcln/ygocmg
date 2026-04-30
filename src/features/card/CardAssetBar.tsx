import { useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { resourceApi } from "../../shared/api/resourceApi";
import { formatError } from "../../shared/utils/format";
import type { CardAssetState, PrimaryType, SpellSubtype } from "../../shared/contracts/card";
import { useAppI18n } from "../../shared/i18n";
import styles from "./CardAssetBar.module.css";

interface CardAssetBarProps {
  workspaceId: string;
  packId: string;
  cardId: string | null;
  cardCode: number;
  packPath: string | null;
  assetState: CardAssetState;
  primaryType: PrimaryType;
  spellSubtype: SpellSubtype | null;
  onAssetChanged: (next: CardAssetState) => void;
  onError: (msg: string) => void;
}

function extractAssetState(result: { status: string; data?: { has_image: boolean; has_field_image: boolean; has_script: boolean } }): CardAssetState | null {
  if (result.status === "ok" && result.data) {
    return {
      has_image: result.data.has_image,
      has_field_image: result.data.has_field_image,
      has_script: result.data.has_script,
    };
  }
  return null;
}

export function CardAssetBar({
  workspaceId,
  packId,
  cardId,
  cardCode,
  packPath,
  assetState,
  primaryType,
  spellSubtype,
  onAssetChanged,
  onError,
}: CardAssetBarProps) {
  const { t } = useAppI18n();
  const isFieldSpell = primaryType === "spell" && spellSubtype === "field";
  const isCreate = cardId === null;
  const [busy, setBusy] = useState(false);
  const [imgKey, setImgKey] = useState(0);

  const imageSrc = assetState.has_image && packPath
    ? convertFileSrc(`${packPath}/pics/${cardCode}.jpg`)
    : null;

  async function handleImportMainImage() {
    if (isCreate || !cardId) return;
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "Images", extensions: ["jpg", "jpeg", "png", "bmp", "webp"] }],
      });
      if (!selected) return;
      setBusy(true);
      const result = await resourceApi.importMainImage({
        workspaceId, packId, cardId, sourcePath: selected,
      });
      const next = extractAssetState(result);
      if (next) {
        onAssetChanged(next);
        setImgKey((k) => k + 1);
      }
    } catch (err) {
      onError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleDeleteMainImage() {
    if (isCreate || !cardId) return;
    setBusy(true);
    try {
      const result = await resourceApi.deleteMainImage({ workspaceId, packId, cardId });
      const next = extractAssetState(result);
      if (next) onAssetChanged(next);
    } catch (err) {
      onError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleImportFieldImage() {
    if (isCreate || !cardId) return;
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "Images", extensions: ["jpg", "jpeg", "png", "bmp", "webp"] }],
      });
      if (!selected) return;
      setBusy(true);
      const result = await resourceApi.importFieldImage({
        workspaceId, packId, cardId, sourcePath: selected,
      });
      const next = extractAssetState(result);
      if (next) onAssetChanged(next);
    } catch (err) {
      onError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleDeleteFieldImage() {
    if (isCreate || !cardId) return;
    setBusy(true);
    try {
      const result = await resourceApi.deleteFieldImage({ workspaceId, packId, cardId });
      const next = extractAssetState(result);
      if (next) onAssetChanged(next);
    } catch (err) {
      onError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleCreateScript() {
    if (isCreate || !cardId) return;
    setBusy(true);
    try {
      const result = await resourceApi.createEmptyScript({ workspaceId, packId, cardId });
      const next = extractAssetState(result);
      if (next) onAssetChanged(next);
    } catch (err) {
      onError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleImportScript() {
    if (isCreate || !cardId) return;
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "Lua Scripts", extensions: ["lua"] }],
      });
      if (!selected) return;
      setBusy(true);
      const result = await resourceApi.importScript({
        workspaceId, packId, cardId, sourcePath: selected,
      });
      const next = extractAssetState(result);
      if (next) onAssetChanged(next);
    } catch (err) {
      onError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleEditScript() {
    if (isCreate || !cardId) return;
    setBusy(true);
    try {
      await resourceApi.openScriptExternal({ workspaceId, packId, cardId });
    } catch (err) {
      onError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleDeleteScript() {
    if (isCreate || !cardId) return;
    setBusy(true);
    try {
      const result = await resourceApi.deleteScript({ workspaceId, packId, cardId });
      const next = extractAssetState(result);
      if (next) onAssetChanged(next);
    } catch (err) {
      onError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className={styles.cardAssetBar}>
      <div className={styles.cardPicPreview}>
        {imageSrc ? (
          <img key={imgKey} src={imageSrc} alt={t("card.asset.cardImageAlt")} />
        ) : (
          t("card.asset.noImage")
        )}
      </div>

      <div className={styles.assetBtnGroup}>
        <span className={styles.assetBtnGroupLabel}>{t("card.asset.image")}</span>
        <button
          type="button"
          className={styles.assetSegBtn}
          disabled={isCreate || busy}
          onClick={() => void handleImportMainImage()}
        >
          {t("action.import")}
        </button>
        {assetState.has_image && (
          <button
            type="button"
            className={`${styles.assetSegBtn} danger`}
            disabled={isCreate || busy}
            onClick={() => void handleDeleteMainImage()}
          >
            {t("action.delete")}
          </button>
        )}
      </div>

      <div className={styles.assetBtnGroup}>
        <span className={styles.assetBtnGroupLabel}>{t("card.asset.script")}</span>
        {assetState.has_script ? (
          <>
            <button
              type="button"
              className={styles.assetSegBtn}
              disabled={isCreate || busy}
              onClick={() => void handleImportScript()}
            >
              {t("action.import")}
            </button>
            <button
              type="button"
              className={styles.assetSegBtn}
              disabled={isCreate || busy}
              onClick={() => void handleEditScript()}
            >
              {t("action.edit")}
            </button>
            <button
              type="button"
              className={`${styles.assetSegBtn} danger`}
              disabled={isCreate || busy}
              onClick={() => void handleDeleteScript()}
            >
              {t("action.delete")}
            </button>
          </>
        ) : (
          <>
            <button
              type="button"
              className={styles.assetSegBtn}
              disabled={isCreate || busy}
              onClick={() => void handleCreateScript()}
            >
              {t("action.create")}
            </button>
            <button
              type="button"
              className={styles.assetSegBtn}
              disabled={isCreate || busy}
              onClick={() => void handleImportScript()}
            >
              {t("action.import")}
            </button>
          </>
        )}
      </div>

      {isFieldSpell && (
        <div className={styles.assetBtnGroup}>
          <span className={styles.assetBtnGroupLabel}>{t("card.asset.field")}</span>
          <button
            type="button"
            className={styles.assetSegBtn}
            disabled={isCreate || busy}
            onClick={() => void handleImportFieldImage()}
          >
            {t("action.import")}
          </button>
          {assetState.has_field_image && (
            <button
              type="button"
              className={`${styles.assetSegBtn} danger`}
              disabled={isCreate || busy}
              onClick={() => void handleDeleteFieldImage()}
            >
              {t("action.delete")}
            </button>
          )}
        </div>
      )}
    </div>
  );
}
