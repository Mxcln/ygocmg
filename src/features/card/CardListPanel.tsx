import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useShellStore } from "../../shared/stores/shellStore";
import { cardApi } from "../../shared/api/cardApi";
import { packApi } from "../../shared/api/packApi";
import type { CardListRow, CardSearchFilters } from "../../shared/contracts/card";
import type { GlobalConfig } from "../../shared/contracts/config";
import type { NoticeTone } from "../../app/NoticeBanner";
import { useAppI18n } from "../../shared/i18n";
import { formatError } from "../../shared/utils/format";
import { CardBrowserPanel } from "./CardBrowserPanel";
import type { CardBrowserPage, CardBrowserQuery } from "./CardBrowserPanel";
import {
  CardAdvancedSearchPanel,
  cardFiltersKey,
  countCardFilters,
} from "./CardAdvancedSearchPanel";
import { useMergedSetnameEntries } from "./useMergedSetnameEntries";
import shared from "../../shared/styles/shared.module.css";
import styles from "./CardListPanel.module.css";

interface CardListPanelProps {
  config: GlobalConfig;
  onEditCard: (card: { id: string; name: string }) => void;
  onNewCard: () => void;
  onNotice: (tone: NoticeTone, title: string, detail: string) => void;
}

export function CardListPanel({ config, onEditCard, onNewCard, onNotice }: CardListPanelProps) {
  const { t } = useAppI18n();
  const queryClient = useQueryClient();
  const [advancedSearchOpen, setAdvancedSearchOpen] = useState(false);
  const [advancedFilters, setAdvancedFilters] = useState<CardSearchFilters | null>(null);
  const [selectedCardIds, setSelectedCardIds] = useState<string[]>([]);
  const [selectionMode, setSelectionMode] = useState(false);
  const [moveDialogOpen, setMoveDialogOpen] = useState(false);
  const [targetPackId, setTargetPackId] = useState("");
  const selectionCleanupRef = useRef<{ packId: string; revision: number } | null>(null);
  const workspaceId = useShellStore((s) => s.workspaceId);
  const activePackId = useShellStore((s) => s.activePackId);
  const openPackIds = useShellStore((s) => s.openPackIds);
  const packMetadataMap = useShellStore((s) => s.packMetadataMap);
  const openDialog = useShellStore((s) => s.openDialog);
  const closeDialog = useShellStore((s) => s.closeDialog);
  const setPackOverviews = useShellStore((s) => s.setPackOverviews);
  const activeMeta = useShellStore((s) =>
    s.activePackId ? s.packMetadataMap[s.activePackId] : null,
  );
  const enabled = !!workspaceId && !!activePackId;
  const languageOrderKey = activeMeta?.display_language_order.join("|") ?? "";
  const defaultLang = activeMeta?.display_language_order[0] || "en-US";
  const activeFilterCount = countCardFilters(advancedFilters);
  const filterKey = useMemo(() => cardFiltersKey(advancedFilters), [advancedFilters]);
  const targetPackOptions = useMemo(
    () =>
      openPackIds
        .filter((packId) => packId !== activePackId)
        .map((packId) => packMetadataMap[packId])
        .filter((metadata) => metadata && metadata.kind === "custom"),
    [activePackId, openPackIds, packMetadataMap],
  );
  const { setnameEntries } = useMergedSetnameEntries({
    workspaceId,
    packId: activePackId,
    language: defaultLang,
    standardLanguage: config.standard_pack_source_language ?? null,
    enabled,
  });

  useEffect(() => {
    selectionCleanupRef.current = null;
    setSelectedCardIds([]);
    setSelectionMode(false);
    setMoveDialogOpen(false);
    setTargetPackId("");
  }, [activePackId]);

  async function loadPage(query: CardBrowserQuery) {
    const page = await cardApi.listCards({
      workspaceId: workspaceId!,
      packId: activePackId!,
      keyword: query.keyword || null,
      filters: advancedFilters,
      sortBy: query.sortBy,
      sortDirection: query.sortDirection,
      page: query.page,
      pageSize: query.pageSize,
    });

    return {
      items: page.items,
      page: page.page,
      page_size: page.page_size,
      total: page.total,
      image_base_path: page.pack_path,
      revision: page.revision,
    };
  }

  function handleRowClick(card: CardListRow) {
    onEditCard({ id: card.id, name: card.name });
  }

  const handlePageLoaded = useCallback(
    (page: CardBrowserPage) => {
      if (!workspaceId || !activePackId || selectedCardIds.length === 0) return;
      const previous = selectionCleanupRef.current;
      if (previous?.packId === activePackId && previous.revision === page.revision) return;
      const cleanupKey = { packId: activePackId, revision: page.revision };
      selectionCleanupRef.current = cleanupKey;

      void (async () => {
        try {
          const allCards = await cardApi.listCards({
            workspaceId,
            packId: activePackId,
            keyword: null,
            filters: null,
            sortBy: "code",
            sortDirection: "asc",
            page: 1,
            pageSize: 100000,
          });
          const latest = selectionCleanupRef.current;
          if (latest?.packId !== cleanupKey.packId || latest.revision !== cleanupKey.revision) {
            return;
          }
          const existingIds = new Set(allCards.items.map((card) => card.id));
          setSelectedCardIds((current) => current.filter((id) => existingIds.has(id)));
        } catch {
          // Selection cleanup is best-effort; normal list/query errors are already shown by the browser panel.
        }
      })();
    },
    [activePackId, selectedCardIds.length, workspaceId],
  );

  async function refreshAfterBatch() {
    setSelectedCardIds([]);
    setSelectionMode(false);
    setMoveDialogOpen(false);
    void queryClient.invalidateQueries({ queryKey: ["cards"] });
    try {
      const overviews = await packApi.listPackOverviews();
      setPackOverviews(overviews);
    } catch {
      // best-effort overview refresh
    }
  }

  async function handleBatchWriteResult(
    result:
      | Awaited<ReturnType<typeof cardApi.bulkDeleteCards>>
      | Awaited<ReturnType<typeof cardApi.moveCards>>,
    successTitle: string,
    successDetail: string,
  ) {
    if (result.status === "ok") {
      closeDialog();
      await refreshAfterBatch();
      onNotice("success", successTitle, successDetail);
      return;
    }

    setMoveDialogOpen(false);
    openDialog({
      kind: "warning",
      title: t("card.batch.warningTitle"),
      message: t("card.batch.warningMessage"),
      confirmLabel: t("card.warning.continue"),
      cancelLabel: t("action.cancel"),
      warnings: result.warnings,
      onConfirm: async () => {
        await cardApi.confirmCardBatchWrite({
          confirmationToken: result.confirmation_token,
        });
        closeDialog();
        await refreshAfterBatch();
        onNotice("success", successTitle, successDetail);
      },
    });
  }

  function handleBulkDelete() {
    if (!workspaceId || !activePackId || selectedCardIds.length === 0) return;
    const count = selectedCardIds.length;
    openDialog({
      kind: "confirm",
      title: t("card.batch.deleteTitle"),
      message: t("card.batch.deleteMessageWithAssets", { count }),
      confirmLabel: t("action.delete"),
      cancelLabel: t("action.cancel"),
      danger: true,
      onConfirm: async () => {
        const result = await cardApi.bulkDeleteCards({
          workspaceId,
          packId: activePackId,
          cardIds: selectedCardIds,
          deleteAssets: true,
        });
        await handleBatchWriteResult(
          result,
          t("card.batch.deleteSuccessTitle"),
          t("card.batch.deleteSuccessDetail", { count }),
        );
      },
    });
  }

  function openMoveDialog() {
    if (targetPackOptions.length === 0) return;
    setTargetPackId((current) => current || targetPackOptions[0]?.id || "");
    setMoveDialogOpen(true);
  }

  async function submitMoveCards() {
    if (!workspaceId || !activePackId || !targetPackId || selectedCardIds.length === 0) return;
    const count = selectedCardIds.length;
    const targetName = packMetadataMap[targetPackId]?.name ?? targetPackId;
    try {
      const result = await cardApi.moveCards({
        workspaceId,
        sourcePackId: activePackId,
        targetPackId,
        cardIds: selectedCardIds,
        moveAssets: true,
      });
      await handleBatchWriteResult(
        result,
        t("card.batch.moveSuccessTitle"),
        t("card.batch.moveSuccessDetail", { count, target: targetName }),
      );
    } catch (err) {
      onNotice("error", t("card.batch.moveFailedTitle"), formatError(err));
    }
  }

  function renderSelectionToolbar() {
    return (
      <>
        <span className={styles.batchCount}>
          {t("card.batch.selectedCount", { count: selectedCardIds.length })}
        </span>
        <button
          type="button"
          className={shared.ghostButton}
          onClick={openMoveDialog}
          disabled={selectedCardIds.length === 0 || targetPackOptions.length === 0}
          title={targetPackOptions.length === 0 ? t("card.batch.noMoveTarget") : undefined}
        >
          {t("card.batch.move")}
        </button>
        <button
          type="button"
          className={shared.dangerButton}
          onClick={handleBulkDelete}
          disabled={selectedCardIds.length === 0}
        >
          {t("card.batch.delete")}
        </button>
      </>
    );
  }

  return (
    <>
      <CardBrowserPanel
        enabled={enabled}
        queryKeyBase={["cards", activePackId, languageOrderKey]}
        queryKeyExtra={[filterKey]}
        resetKey={filterKey}
        loadPage={loadPage}
        onOpenCard={handleRowClick}
        onNewCard={onNewCard}
        onPageLoaded={handlePageLoaded}
        selectedCardIds={selectedCardIds}
        onSelectionChange={setSelectedCardIds}
        selectionMode={selectionMode}
        onSelectionModeChange={setSelectionMode}
        selectionToolbar={renderSelectionToolbar()}
        toolbarExtra={
          <button
            type="button"
            className={shared.ghostButton}
            onClick={() => setAdvancedSearchOpen((open) => !open)}
            aria-expanded={advancedSearchOpen}
          >
            {activeFilterCount > 0
              ? t("standard.search.filtersWithCount", { count: activeFilterCount })
              : t("standard.search.filters")}
          </button>
        }
        toolbarPanel={
          <CardAdvancedSearchPanel
            open={advancedSearchOpen}
            filters={advancedFilters}
            setnameEntries={setnameEntries}
            onChange={setAdvancedFilters}
            onClose={() => setAdvancedSearchOpen(false)}
          />
        }
        emptyTitle={t("card.list.noCards")}
        emptyHint={activeFilterCount > 0 ? t("standard.tryAnotherFilter") : t("card.list.createHint")}
      />

      {moveDialogOpen && (
        <div className={styles.moveLayer}>
          <div className={styles.moveBackdrop} onClick={() => setMoveDialogOpen(false)} />
          <section className={styles.moveDialog} role="dialog" aria-modal="true">
            <header className={shared.modalHeader}>
              <div>
                <h2>{t("card.batch.moveTitle")}</h2>
                <p className={styles.moveSummary}>
                  {t("card.batch.moveMessage", { count: selectedCardIds.length })}
                </p>
              </div>
              <button
                type="button"
                className={shared.modalCloseButton}
                onClick={() => setMoveDialogOpen(false)}
              >
                {t("action.close")}
              </button>
            </header>
            <div className={styles.moveBody}>
              <label className={shared.field}>
                <span>{t("card.batch.targetPack")}</span>
                <select
                  value={targetPackId}
                  onChange={(event) => setTargetPackId(event.target.value)}
                >
                  {targetPackOptions.map((metadata) => (
                    <option key={metadata.id} value={metadata.id}>
                      {metadata.name}
                    </option>
                  ))}
                </select>
              </label>
            </div>
            <div className={styles.moveActions}>
              <button
                type="button"
                className={shared.ghostButton}
                onClick={() => setMoveDialogOpen(false)}
              >
                {t("action.cancel")}
              </button>
              <button
                type="button"
                className={shared.primaryButton}
                onClick={() => void submitMoveCards()}
                disabled={!targetPackId}
              >
                {t("card.batch.move")}
              </button>
            </div>
          </section>
        </div>
      )}
    </>
  );
}
