import { useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { resourceApi } from "../../shared/api/resourceApi";
import { formatError } from "../../shared/utils/format";
import type { CardAssetState, PrimaryType, SpellSubtype } from "../../shared/contracts/card";

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
    } catch (err: unknown) {
      const appErr = err as { code?: string };
      if (appErr.code === "resource.external_editor_not_configured") {
        onError("External text editor is not configured. Set it in Global Settings.");
      } else {
        onError(formatError(err));
      }
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
    <div className="card-asset-bar">
      <div className="card-pic-preview">
        {imageSrc ? (
          <img key={imgKey} src={imageSrc} alt="Card" />
        ) : (
          "No Image"
        )}
      </div>

      <div className="asset-btn-group">
        <span className="asset-btn-group-label">Image</span>
        <button
          type="button"
          className="asset-seg-btn"
          disabled={isCreate || busy}
          onClick={() => void handleImportMainImage()}
        >
          Import
        </button>
        {assetState.has_image && (
          <button
            type="button"
            className="asset-seg-btn danger"
            disabled={isCreate || busy}
            onClick={() => void handleDeleteMainImage()}
          >
            Delete
          </button>
        )}
      </div>

      <div className="asset-btn-group">
        <span className="asset-btn-group-label">Script</span>
        {assetState.has_script ? (
          <>
            <button
              type="button"
              className="asset-seg-btn"
              disabled={isCreate || busy}
              onClick={() => void handleImportScript()}
            >
              Import
            </button>
            <button
              type="button"
              className="asset-seg-btn"
              disabled={isCreate || busy}
              onClick={() => void handleEditScript()}
            >
              Edit
            </button>
            <button
              type="button"
              className="asset-seg-btn danger"
              disabled={isCreate || busy}
              onClick={() => void handleDeleteScript()}
            >
              Delete
            </button>
          </>
        ) : (
          <>
            <button
              type="button"
              className="asset-seg-btn"
              disabled={isCreate || busy}
              onClick={() => void handleCreateScript()}
            >
              Create
            </button>
            <button
              type="button"
              className="asset-seg-btn"
              disabled={isCreate || busy}
              onClick={() => void handleImportScript()}
            >
              Import
            </button>
          </>
        )}
      </div>

      {isFieldSpell && (
        <div className="asset-btn-group">
          <span className="asset-btn-group-label">Field</span>
          <button
            type="button"
            className="asset-seg-btn"
            disabled={isCreate || busy}
            onClick={() => void handleImportFieldImage()}
          >
            Import
          </button>
          {assetState.has_field_image && (
            <button
              type="button"
              className="asset-seg-btn danger"
              disabled={isCreate || busy}
              onClick={() => void handleDeleteFieldImage()}
            >
              Delete
            </button>
          )}
        </div>
      )}
    </div>
  );
}
