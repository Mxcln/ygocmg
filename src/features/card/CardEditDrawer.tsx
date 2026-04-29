import { useCallback, useEffect, useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { cardApi } from "../../shared/api/cardApi";
import { useShellStore } from "../../shared/stores/shellStore";
import { formatError } from "../../shared/utils/format";
import type { GlobalConfig } from "../../shared/contracts/config";
import type { CardEntity, CardAssetState } from "../../shared/contracts/card";
import type { CardDetail } from "../../shared/contracts/card";
import type { ValidationIssue } from "../../shared/contracts/common";
import { CardAssetBar } from "./CardAssetBar";
import { CardInfoForm } from "./CardInfoForm";
import { CardTextForm } from "./CardTextForm";

interface CardEditDrawerProps {
  packId: string;
  workspaceId: string;
  cardId: string | null;
  config: GlobalConfig;
  onClose: () => void;
  onSaved: () => void;
}

type DrawerTab = "text" | "info";

const EMPTY_ASSET_STATE: CardAssetState = {
  has_image: false,
  has_script: false,
  has_field_image: false,
};

const EMPTY_STRINGS = Array.from({ length: 16 }, () => "");

function makeBlankCard(suggestedCode: number, defaultLang: string): CardEntity {
  return {
    id: "",
    code: suggestedCode,
    alias: 0,
    setcode: 0,
    ot: "custom",
    category: 0,
    primary_type: "monster",
    texts: {
      [defaultLang]: { name: "New Card", desc: "", strings: [...EMPTY_STRINGS] },
    },
    monster_flags: ["normal"],
    atk: 0,
    def: 0,
    race: "warrior",
    attribute: "earth",
    level: 4,
    pendulum: null,
    link: null,
    spell_subtype: null,
    trap_subtype: null,
    created_at: "",
    updated_at: "",
  };
}

export function CardEditDrawer({
  packId,
  workspaceId,
  cardId,
  config,
  onClose,
  onSaved,
}: CardEditDrawerProps) {
  const isCreate = cardId === null;
  const [draft, setDraft] = useState<CardEntity | null>(null);
  const [assetState, setAssetState] = useState<CardAssetState>(EMPTY_ASSET_STATE);
  const [packPath, setPackPath] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<DrawerTab>("text");
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [warnings, setWarnings] = useState<ValidationIssue[]>([]);
  const [closing, setClosing] = useState(false);
  const drawerRef = useRef<HTMLDivElement>(null);
  const queryClient = useQueryClient();

  const activeMeta = useShellStore((s) =>
    s.activePackId ? s.packMetadataMap[s.activePackId] : null,
  );
  const openDialog = useShellStore((s) => s.openDialog);
  const closeDialog = useShellStore((s) => s.closeDialog);
  const updatePackMetadataInStore = useShellStore((s) => s.updatePackMetadata);
  const displayLanguageOrder = activeMeta?.display_language_order ?? [];

  const { data: cardDetail, isLoading: loadingDetail } = useQuery({
    queryKey: ["card", packId, cardId],
    queryFn: () =>
      cardApi.getCard({ workspaceId, packId, cardId: cardId! }),
    enabled: !isCreate && !!cardId,
  });

  const defaultLang = displayLanguageOrder[0] || "en-US";

  useEffect(() => {
    if (isCreate) {
      cardApi
        .suggestCardCode({ workspaceId, packId, preferredStart: null })
        .then((result) => {
          setDraft(makeBlankCard(result.suggested_code ?? 100000000, defaultLang));
          if (result.warnings.length > 0) setWarnings(result.warnings);
        })
        .catch((err) => {
          setDraft(makeBlankCard(100000000, defaultLang));
          setErrorMsg(formatError(err));
        });
    }
  }, [isCreate, workspaceId, packId, defaultLang]);

  useEffect(() => {
    if (cardDetail) {
      setDraft(cardDetail.card);
      setAssetState(cardDetail.asset_state);
      setPackPath(cardDetail.pack_path);
    }
  }, [cardDetail]);

  const handleChange = useCallback(
    (patch: Partial<CardEntity>) => {
      setDraft((prev) => (prev ? { ...prev, ...patch } : prev));
    },
    [],
  );

  const handleAssetChanged = useCallback(
    (next: CardAssetState) => {
      setAssetState(next);
      void queryClient.invalidateQueries({ queryKey: ["cards"] });
    },
    [queryClient],
  );

  function handleAnimatedClose() {
    setClosing(true);
    setTimeout(() => {
      onClose();
    }, 180);
  }

  function applySavedDetail(detail: CardDetail) {
    setDraft(detail.card);
    setAssetState(detail.asset_state);
    setPackPath(detail.pack_path);
    queryClient.setQueryData(["card", packId, detail.card.id], detail);
    void queryClient.invalidateQueries({ queryKey: ["card", packId, detail.card.id] });
    void queryClient.invalidateQueries({ queryKey: ["cards"] });
    if (activeMeta) {
      updatePackMetadataInStore(packId, { ...activeMeta, updated_at: detail.card.updated_at });
    }
  }

  async function handleSave() {
    if (!draft) return;
    setSaving(true);
    setErrorMsg(null);
    setWarnings([]);

    try {
      const cardPayload = {
        code: draft.code,
        alias: draft.alias,
        setcode: draft.setcode,
        ot: draft.ot,
        category: draft.category,
        primary_type: draft.primary_type,
        texts: draft.texts,
        monster_flags: draft.monster_flags,
        atk: draft.atk,
        def: draft.def,
        race: draft.race,
        attribute: draft.attribute,
        level: draft.level,
        pendulum: draft.pendulum,
        link: draft.link,
        spell_subtype: draft.spell_subtype,
        trap_subtype: draft.trap_subtype,
      };

      const result = isCreate
        ? await cardApi.createCard({ workspaceId, packId, card: cardPayload })
        : await cardApi.updateCard({
            workspaceId,
            packId,
            cardId: cardId!,
            card: cardPayload,
          });

      if (result.status === "ok") {
        if (result.warnings.length > 0) {
          setWarnings(result.warnings);
        }
        applySavedDetail(result.data);
        onSaved();
        handleAnimatedClose();
      } else {
        setErrorMsg(null);
        openDialog({
          kind: "warning",
          title: isCreate ? "Review card warnings" : "Review save warnings",
          message: "This write produced warnings. Continue to apply the change?",
          confirmLabel: "Continue",
          cancelLabel: "Cancel",
          warnings: result.warnings,
          onConfirm: async () => {
            try {
              const detail = await cardApi.confirmCardWrite({
                confirmationToken: result.confirmation_token,
              });
              setWarnings([]);
              applySavedDetail(detail);
              closeDialog();
              onSaved();
              handleAnimatedClose();
            } catch (err) {
              setErrorMsg(formatError(err));
            }
          },
        });
        setSaving(false);
        return;
      }
    } catch (err) {
      setErrorMsg(formatError(err));
    } finally {
      setSaving(false);
    }
  }

  async function handleDelete() {
    if (!cardId) return;
    const deleteCardId = cardId;
    setErrorMsg(null);
    openDialog({
      kind: "confirm",
      title: "Delete card",
      message: "This card will be permanently removed from the pack.",
      confirmLabel: "Delete",
      cancelLabel: "Cancel",
      danger: true,
      onConfirm: async () => {
        setDeleting(true);
        setErrorMsg(null);
        try {
          const result = await cardApi.deleteCard({ workspaceId, packId, cardId: deleteCardId });
          if (result.status !== "ok") {
            throw new Error("Delete card returned an unsupported confirmation state.");
          }
          closeDialog();
          onSaved();
          handleAnimatedClose();
        } catch (err) {
          setErrorMsg(formatError(err));
        } finally {
          setDeleting(false);
        }
      },
    });
  }

  const showLoading = !isCreate && loadingDetail && !draft;

  return (
    <>
      <div className="card-edit-backdrop" onClick={handleAnimatedClose} />
      <div
        ref={drawerRef}
        className={`card-edit-drawer ${closing ? "closing" : ""}`}
      >
        <div className="card-edit-header">
          <div className="card-edit-header-left">
            <button
              type="button"
              className="ghost-button"
              onClick={handleAnimatedClose}
            >
              Close
            </button>
            {!isCreate && (
              <button
                type="button"
                className="danger-button"
                onClick={() => void handleDelete()}
                disabled={deleting}
              >
                {deleting ? "Deleting..." : "Delete"}
              </button>
            )}
          </div>
          <div className="card-edit-header-spacer" />
          <button
            type="button"
            className="primary-button"
            onClick={() => void handleSave()}
            disabled={saving || !draft}
          >
            {saving ? "Saving..." : isCreate ? "Create" : "Save"}
          </button>
        </div>

        {warnings.length > 0 && (
          <div className="card-edit-warnings">
            <ul>
              {warnings.map((w, i) => (
                <li key={i}>{w.code}: {JSON.stringify(w.params)}</li>
              ))}
            </ul>
          </div>
        )}

        {errorMsg && (
          <div className="card-edit-error">{errorMsg}</div>
        )}

        {showLoading ? (
          <div className="card-list-empty">
            <p>Loading card...</p>
          </div>
        ) : draft ? (
          <div className="card-edit-body">
            <CardAssetBar
              workspaceId={workspaceId}
              packId={packId}
              cardId={isCreate ? null : cardId}
              cardCode={draft.code}
              packPath={packPath}
              assetState={assetState}
              primaryType={draft.primary_type}
              spellSubtype={draft.spell_subtype}
              onAssetChanged={handleAssetChanged}
              onError={(msg) => setErrorMsg(msg)}
            />
            <div className="card-form-area">
              <div className="card-form-tabs">
                <button
                  type="button"
                  className={`card-form-tab ${activeTab === "text" ? "active" : ""}`}
                  onClick={() => setActiveTab("text")}
                >
                  Text
                </button>
                <button
                  type="button"
                  className={`card-form-tab ${activeTab === "info" ? "active" : ""}`}
                  onClick={() => setActiveTab("info")}
                >
                  Info
                </button>
              </div>
              <div className="card-form-content">
                {activeTab === "text" ? (
                  <CardTextForm
                    draft={draft}
                    catalog={config.text_language_catalog}
                    displayLanguageOrder={displayLanguageOrder}
                    onChange={handleChange}
                    onConfirmDeleteLanguage={(language, onConfirm) => {
                      openDialog({
                        kind: "confirm",
                        title: "Delete language text",
                        message: `Delete card text for ${language}? This cannot be undone.`,
                        confirmLabel: "Delete",
                        cancelLabel: "Cancel",
                        danger: true,
                        onConfirm: () => {
                          onConfirm();
                          closeDialog();
                        },
                      });
                    }}
                  />
                ) : (
                  <CardInfoForm draft={draft} onChange={handleChange} />
                )}
              </div>
            </div>
          </div>
        ) : null}
      </div>
    </>
  );
}
