import { useState } from "react";
import { useQuery, useQueryClient, keepPreviousData } from "@tanstack/react-query";
import { useShellStore } from "../../shared/stores/shellStore";
import { cardApi } from "../../shared/api/cardApi";
import type { CardSortField, SortDirection, CardListRow, CardListPage } from "../../shared/contracts/card";

const PAGE_SIZE = 20;

function formatStat(val: number | null): string {
  if (val === null) return "";
  if (val === -2) return "?";
  return String(val);
}

function subtypeTagClass(tag: string, primaryType: string): string {
  if (primaryType === "monster") {
    return `subtype-tag flag-${tag.toLowerCase().replace(/[^a-z]/g, "")}`;
  }
  return `subtype-tag ${primaryType}`;
}

interface CardListPanelProps {
  onEditCard: (cardId: string) => void;
  onNewCard: () => void;
}

export function CardListPanel({ onEditCard, onNewCard }: CardListPanelProps) {
  const workspaceId = useShellStore((s) => s.workspaceId);
  const activePackId = useShellStore((s) => s.activePackId);

  const [keyword, setKeyword] = useState("");
  const [sortBy, setSortBy] = useState<CardSortField>("code");
  const [sortDirection, setSortDirection] = useState<SortDirection>("asc");
  const [page, setPage] = useState(1);

  const enabled = !!workspaceId && !!activePackId;

  const { data, isLoading, error } = useQuery<CardListPage>({
    queryKey: ["cards", activePackId, keyword, sortBy, sortDirection, page],
    queryFn: () =>
      cardApi.listCards({
        workspaceId: workspaceId!,
        packId: activePackId!,
        keyword: keyword || null,
        sortBy,
        sortDirection,
        page,
        pageSize: PAGE_SIZE,
      }),
    enabled,
    placeholderData: keepPreviousData,
  });

  const items = data?.items ?? [];
  const total = data?.total ?? 0;
  const totalPages = Math.max(1, Math.ceil(total / PAGE_SIZE));

  function handleSearchChange(value: string) {
    setKeyword(value);
    setPage(1);
  }

  function handleSortChange(value: string) {
    const [field, dir] = value.split(":") as [CardSortField, SortDirection];
    setSortBy(field);
    setSortDirection(dir);
    setPage(1);
  }

  function handleRowClick(card: CardListRow) {
    onEditCard(card.id);
  }

  return (
    <>
      <div className="card-list-toolbar">
        <input
          className="card-search-input"
          type="text"
          placeholder="Search cards..."
          value={keyword}
          onChange={(e) => handleSearchChange(e.target.value)}
        />
        <select
          className="card-sort-select"
          value={`${sortBy}:${sortDirection}`}
          onChange={(e) => handleSortChange(e.target.value)}
        >
          <option value="code:asc">Code Asc</option>
          <option value="code:desc">Code Desc</option>
          <option value="name:asc">Name A-Z</option>
          <option value="name:desc">Name Z-A</option>
        </select>
        <button type="button" className="primary-button" onClick={onNewCard}>
          + New Card
        </button>
      </div>

      {isLoading && items.length === 0 ? (
        <div className="card-list-empty">
          <p>Loading cards...</p>
        </div>
      ) : error ? (
        <div className="card-list-empty">
          <p>Failed to load cards.</p>
        </div>
      ) : items.length === 0 ? (
        <div className="card-list-empty">
          <p>No cards yet.</p>
          <p>Click &quot;+ New Card&quot; to create one.</p>
        </div>
      ) : (
        <>
          <div className="card-list-header">
            <span />
            <span>Code</span>
            <span>Name</span>
            <span>Type</span>
            <span>Subtype</span>
            <span>ATK</span>
            <span>DEF</span>
            <span>Lv</span>
            <span />
          </div>
          <div className="card-list-body">
            {items.map((card: CardListRow) => (
              <div
                key={card.id}
                className="card-list-row"
                onClick={() => handleRowClick(card)}
              >
                <div className="card-list-thumb">
                  <svg width="16" height="20" viewBox="0 0 16 20" fill="none" stroke="currentColor" strokeWidth="1">
                    <rect x="1" y="1" width="14" height="18" rx="1.5" />
                    <rect x="3" y="3" width="10" height="8" rx="0.5" />
                  </svg>
                </div>
                <span className="card-list-code">{card.code}</span>
                <span className="card-list-name" title={card.name}>
                  {card.name || "(no name)"}
                </span>
                <span className={`card-type-badge ${card.primary_type}`}>
                  {card.primary_type}
                </span>
                <span className="card-list-subtype">
                  {card.subtype_display.split(" / ").map((tag) => (
                    <span key={tag} className={subtypeTagClass(tag, card.primary_type)}>{tag}</span>
                  ))}
                </span>
                <span className="card-list-stat">{card.atk !== null ? formatStat(card.atk) : ""}</span>
                <span className="card-list-stat">{card.def !== null ? formatStat(card.def) : ""}</span>
                <span className="card-list-stat">{card.level !== null ? String(card.level) : ""}</span>
                <span className="card-list-assets">
                  <svg width="12" height="12" viewBox="0 0 12 12" className={card.has_image ? "active" : ""}>
                    <rect x="0.5" y="0.5" width="11" height="11" rx="1.5" fill="none" stroke="currentColor" strokeWidth="1" />
                    <circle cx="4" cy="4.5" r="1.2" fill="currentColor" />
                    <path d="M1 10l3-4 2 2 2-3 3 5H1z" fill="currentColor" opacity="0.5" />
                  </svg>
                  <svg width="12" height="12" viewBox="0 0 12 12" className={card.has_script ? "active" : ""}>
                    <path d="M2 1h5l3 3v7a1 1 0 01-1 1H2a1 1 0 01-1-1V2a1 1 0 011-1z" fill="none" stroke="currentColor" strokeWidth="1" />
                    <path d="M3.5 6h5M3.5 8h3" stroke="currentColor" strokeWidth="0.8" />
                  </svg>
                </span>
              </div>
            ))}
          </div>

          {totalPages > 1 && (
            <div className="card-list-pagination">
              <button
                type="button"
                disabled={page <= 1}
                onClick={() => setPage((p) => Math.max(1, p - 1))}
              >
                Prev
              </button>
              <span>
                {page} / {totalPages}
              </span>
              <button
                type="button"
                disabled={page >= totalPages}
                onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
              >
                Next
              </button>
            </div>
          )}
        </>
      )}
    </>
  );
}
