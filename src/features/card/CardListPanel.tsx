import { useMemo, useState } from "react";
import { useShellStore } from "../../shared/stores/shellStore";
import { cardApi } from "../../shared/api/cardApi";
import type { CardListRow, CardSearchFilters, CardSortField } from "../../shared/contracts/card";
import type { GlobalConfig } from "../../shared/contracts/config";
import { useAppI18n } from "../../shared/i18n";
import { CardBrowserPanel } from "./CardBrowserPanel";
import type { BrowserSortField, CardBrowserQuery } from "./CardBrowserPanel";
import {
  CardAdvancedSearchPanel,
  cardFiltersKey,
  countCardFilters,
} from "./CardAdvancedSearchPanel";
import { useMergedSetnameEntries } from "./useMergedSetnameEntries";
import shared from "../../shared/styles/shared.module.css";

interface CardListPanelProps {
  config: GlobalConfig;
  onEditCard: (cardId: string) => void;
  onNewCard: () => void;
}

function toCustomSortField(field: BrowserSortField): CardSortField {
  return field === "name" ? "name" : "code";
}

export function CardListPanel({ config, onEditCard, onNewCard }: CardListPanelProps) {
  const { t } = useAppI18n();
  const [advancedSearchOpen, setAdvancedSearchOpen] = useState(false);
  const [advancedFilters, setAdvancedFilters] = useState<CardSearchFilters | null>(null);
  const workspaceId = useShellStore((s) => s.workspaceId);
  const activePackId = useShellStore((s) => s.activePackId);
  const activeMeta = useShellStore((s) =>
    s.activePackId ? s.packMetadataMap[s.activePackId] : null,
  );
  const enabled = !!workspaceId && !!activePackId;
  const languageOrderKey = activeMeta?.display_language_order.join("|") ?? "";
  const defaultLang = activeMeta?.display_language_order[0] || "en-US";
  const activeFilterCount = countCardFilters(advancedFilters);
  const filterKey = useMemo(() => cardFiltersKey(advancedFilters), [advancedFilters]);
  const { setnameEntries } = useMergedSetnameEntries({
    workspaceId,
    packId: activePackId,
    language: defaultLang,
    standardLanguage: config.standard_pack_source_language ?? null,
    enabled,
  });

  async function loadPage(query: CardBrowserQuery) {
    const page = await cardApi.listCards({
      workspaceId: workspaceId!,
      packId: activePackId!,
      keyword: query.keyword || null,
      filters: advancedFilters,
      sortBy: toCustomSortField(query.sortBy),
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
    onEditCard(card.id);
  }

  return (
    <CardBrowserPanel
      enabled={enabled}
      queryKeyBase={["cards", activePackId, languageOrderKey]}
      queryKeyExtra={[filterKey]}
      resetKey={filterKey}
      loadPage={loadPage}
      onOpenCard={handleRowClick}
      onNewCard={onNewCard}
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
  );
}
