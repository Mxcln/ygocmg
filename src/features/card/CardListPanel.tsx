import { useShellStore } from "../../shared/stores/shellStore";
import { cardApi } from "../../shared/api/cardApi";
import type { CardListRow, CardSortField } from "../../shared/contracts/card";
import { useAppI18n } from "../../shared/i18n";
import { CardBrowserPanel } from "./CardBrowserPanel";
import type { BrowserSortField, CardBrowserQuery } from "./CardBrowserPanel";

interface CardListPanelProps {
  onEditCard: (cardId: string) => void;
  onNewCard: () => void;
}

function toCustomSortField(field: BrowserSortField): CardSortField {
  return field === "name" ? "name" : "code";
}

export function CardListPanel({ onEditCard, onNewCard }: CardListPanelProps) {
  const { t } = useAppI18n();
  const workspaceId = useShellStore((s) => s.workspaceId);
  const activePackId = useShellStore((s) => s.activePackId);
  const activeMeta = useShellStore((s) =>
    s.activePackId ? s.packMetadataMap[s.activePackId] : null,
  );
  const enabled = !!workspaceId && !!activePackId;
  const languageOrderKey = activeMeta?.display_language_order.join("|") ?? "";

  async function loadPage(query: CardBrowserQuery) {
    const page = await cardApi.listCards({
      workspaceId: workspaceId!,
      packId: activePackId!,
      keyword: query.keyword || null,
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
      loadPage={loadPage}
      onOpenCard={handleRowClick}
      onNewCard={onNewCard}
      emptyTitle={t("card.list.noCards")}
      emptyHint={t("card.list.createHint")}
    />
  );
}
