import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { cardApi } from "../../shared/api/cardApi";
import { stringsApi } from "../../shared/api/stringsApi";
import { standardPackApi } from "../../shared/api/standardPackApi";
import { useShellStore } from "../../shared/stores/shellStore";
import { formatError, formatValidationIssue } from "../../shared/utils/format";
import { formatIssueDetail } from "../../shared/utils/messages";
import type { GlobalConfig } from "../../shared/contracts/config";
import type { CardEntity, CardAssetState } from "../../shared/contracts/card";
import type { CardDetail } from "../../shared/contracts/card";
import type { ValidationIssue } from "../../shared/contracts/common";
import { useAppI18n } from "../../shared/i18n";
import shared from "../../shared/styles/shared.module.css";
import styles from "./CardEditDrawer.module.css";
import { CardAssetBar } from "./CardAssetBar";
import { CardInfoForm, type SetnameEntry } from "./CardInfoForm";
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
    setcodes: [],
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
  const { t } = useAppI18n();
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
  const standardSetnameLanguage = config.standard_pack_source_language ?? null;

  const { data: cardDetail, isLoading: loadingDetail } = useQuery({
    queryKey: ["card", packId, cardId],
    queryFn: () =>
      cardApi.getCard({ workspaceId, packId, cardId: cardId! }),
    enabled: !isCreate && !!cardId,
  });

  const defaultLang = displayLanguageOrder[0] || "en-US";

  const { data: standardSetnames } = useQuery({
    queryKey: ["standard-setnames", standardSetnameLanguage],
    queryFn: () => standardPackApi.listSetnames({ language: standardSetnameLanguage }),
    staleTime: 5 * 60 * 1000,
  });

  const { data: packSetnamesPage } = useQuery({
    queryKey: ["pack-setnames", packId],
    queryFn: () =>
      stringsApi.listPackStrings({
        workspaceId,
        packId,
        language: defaultLang,
        kindFilter: "setname",
        keyword: null,
        keyFilter: null,
        page: 1,
        pageSize: 10000,
      }),
    staleTime: 30_000,
  });

  const setnameEntries = useMemo<SetnameEntry[]>(() => {
    const entries: SetnameEntry[] = [];
    if (packSetnamesPage?.items) {
      for (const item of packSetnamesPage.items) {
        entries.push({ key: item.key, name: item.value, source: "pack" });
      }
    }
    if (standardSetnames) {
      for (const item of standardSetnames) {
        entries.push({ key: item.key, name: item.value, source: "standard" });
      }
    }
    return entries;
  }, [standardSetnames, packSetnamesPage]);

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
        setcodes: draft.setcodes,
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
          title: isCreate ? t("card.warning.createTitle") : t("card.warning.saveTitle"),
          message: t("card.warning.message"),
          confirmLabel: t("card.warning.continue"),
          cancelLabel: t("action.cancel"),
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
      title: t("card.delete.title"),
      message: t("card.delete.message"),
      confirmLabel: t("action.delete"),
      cancelLabel: t("action.cancel"),
      danger: true,
      onConfirm: async () => {
        setDeleting(true);
        setErrorMsg(null);
        try {
          const result = await cardApi.deleteCard({ workspaceId, packId, cardId: deleteCardId });
          if (result.status !== "ok") {
            throw new Error(t("card.delete.unsupportedState"));
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
      <div className={styles.cardEditBackdrop} onClick={handleAnimatedClose} />
      <div
        ref={drawerRef}
        className={`${styles.cardEditDrawer} ${closing ? "closing" : ""}`}
      >
        <div className={styles.cardEditHeader}>
          <div className={styles.cardEditHeaderLeft}>
            <button
              type="button"
              className={shared.ghostButton}
              onClick={handleAnimatedClose}
            >
              {t("action.close")}
            </button>
            {!isCreate && (
              <button
                type="button"
                className={shared.dangerButton}
                onClick={() => void handleDelete()}
                disabled={deleting}
              >
                {deleting ? t("card.deleting") : t("action.delete")}
              </button>
            )}
          </div>
          <div className={styles.cardEditHeaderSpacer} />
          <button
            type="button"
            className={shared.primaryButton}
            onClick={() => void handleSave()}
            disabled={saving || !draft}
          >
            {saving ? t("pack.metadata.saving") : isCreate ? t("action.create") : t("action.save")}
          </button>
        </div>

        {warnings.length > 0 && (
          <div className={styles.cardEditWarnings}>
            <ul>
              {warnings.map((w, i) => (
                <li key={i}>
                  <span>{formatValidationIssue(w)}</span>
                  {formatIssueDetail(w) && <small>{formatIssueDetail(w)}</small>}
                </li>
              ))}
            </ul>
          </div>
        )}

        {errorMsg && (
          <div className={styles.cardEditError}>{errorMsg}</div>
        )}

        {showLoading ? (
          <div className={shared.cardListEmpty}>
            <p>{t("card.loading")}</p>
          </div>
        ) : draft ? (
          <div className={styles.cardEditBody}>
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
            <div className={styles.cardFormArea}>
              <div className={styles.cardFormTabs}>
                <button
                  type="button"
                  className={`${styles.cardFormTab} ${activeTab === "text" ? "active" : ""}`}
                  onClick={() => setActiveTab("text")}
                >
                  {t("card.tab.text")}
                </button>
                <button
                  type="button"
                  className={`${styles.cardFormTab} ${activeTab === "info" ? "active" : ""}`}
                  onClick={() => setActiveTab("info")}
                >
                  {t("card.tab.info")}
                </button>
              </div>
              <div className={styles.cardFormContent}>
                {activeTab === "text" ? (
                  <CardTextForm
                    draft={draft}
                    catalog={config.text_language_catalog}
                    displayLanguageOrder={displayLanguageOrder}
                    onChange={handleChange}
                    onConfirmDeleteLanguage={(language, onConfirm) => {
                      openDialog({
                        kind: "confirm",
                        title: t("card.text.deleteLanguageTitle"),
                        message: t("card.text.deleteLanguageMessage", { language }),
                        confirmLabel: t("action.delete"),
                        cancelLabel: t("action.cancel"),
                        danger: true,
                        onConfirm: () => {
                          onConfirm();
                          closeDialog();
                        },
                      });
                    }}
                  />
                ) : (
                  <CardInfoForm draft={draft} onChange={handleChange} setnameEntries={setnameEntries} />
                )}
              </div>
            </div>
          </div>
        ) : null}
      </div>
    </>
  );
}
