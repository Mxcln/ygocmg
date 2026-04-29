import { useState } from "react";
import { useQuery, keepPreviousData } from "@tanstack/react-query";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { CardListRow, SortDirection } from "../../shared/contracts/card";

const PAGE_SIZE = 6;

export type BrowserSortField = "code" | "name" | "type";

export interface CardBrowserQuery {
  keyword: string;
  sortBy: BrowserSortField;
  sortDirection: SortDirection;
  page: number;
  pageSize: number;
}

export interface CardBrowserPage {
  items: CardListRow[];
  page: number;
  page_size: number;
  total: number;
  image_base_path: string | null;
  revision: number;
}

export interface CardBrowserSortOption {
  field: BrowserSortField;
  direction: SortDirection;
  label: string;
}

interface CardBrowserPanelProps {
  enabled: boolean;
  queryKeyBase: readonly unknown[];
  loadPage: (query: CardBrowserQuery) => Promise<CardBrowserPage>;
  onOpenCard: (card: CardListRow) => void;
  onNewCard?: () => void;
  newCardLabel?: string;
  sortOptions?: CardBrowserSortOption[];
  emptyTitle: string;
  emptyHint?: string;
  loadingText?: string;
  errorText?: string;
}

const DEFAULT_SORT_OPTIONS: CardBrowserSortOption[] = [
  { field: "code", direction: "asc", label: "Code Asc" },
  { field: "code", direction: "desc", label: "Code Desc" },
  { field: "name", direction: "asc", label: "Name A-Z" },
  { field: "name", direction: "desc", label: "Name Z-A" },
];

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

function cardImageSrc(basePath: string, code: number, revision: number): string {
  return `${convertFileSrc(`${basePath}/pics/${code}.jpg`)}?v=${revision}`;
}

function sortValue(field: BrowserSortField, direction: SortDirection): string {
  return `${field}:${direction}`;
}

function buildPageNumbers(current: number, total: number): (number | "...")[] {
  if (total <= 7) {
    return Array.from({ length: total }, (_, i) => i + 1);
  }

  const pages: (number | "...")[] = [1];

  let rangeStart = Math.max(2, current - 1);
  let rangeEnd = Math.min(total - 1, current + 1);

  if (current <= 3) {
    rangeStart = 2;
    rangeEnd = Math.min(5, total - 1);
  } else if (current >= total - 2) {
    rangeStart = Math.max(2, total - 4);
    rangeEnd = total - 1;
  }

  if (rangeStart > 2) pages.push("...");
  for (let i = rangeStart; i <= rangeEnd; i++) pages.push(i);
  if (rangeEnd < total - 1) pages.push("...");

  pages.push(total);
  return pages;
}

export function CardBrowserPanel({
  enabled,
  queryKeyBase,
  loadPage,
  onOpenCard,
  onNewCard,
  newCardLabel = "+ New Card",
  sortOptions = DEFAULT_SORT_OPTIONS,
  emptyTitle,
  emptyHint,
  loadingText = "Loading cards...",
  errorText = "Failed to load cards.",
}: CardBrowserPanelProps) {
  const [keyword, setKeyword] = useState("");
  const [sortBy, setSortBy] = useState<BrowserSortField>("code");
  const [sortDirection, setSortDirection] = useState<SortDirection>("asc");
  const [page, setPage] = useState(1);

  const { data, isLoading, error } = useQuery<CardBrowserPage>({
    queryKey: [...queryKeyBase, keyword, sortBy, sortDirection, page],
    queryFn: () =>
      loadPage({
        keyword,
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
  const imageBasePath = data?.image_base_path ?? null;
  const revision = data?.revision ?? 0;
  const totalPages = Math.max(1, Math.ceil(total / PAGE_SIZE));

  function handleSearchChange(value: string) {
    setKeyword(value);
    setPage(1);
  }

  function handleSortChange(value: string) {
    const [field, direction] = value.split(":") as [BrowserSortField, SortDirection];
    setSortBy(field);
    setSortDirection(direction);
    setPage(1);
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
          value={sortValue(sortBy, sortDirection)}
          onChange={(e) => handleSortChange(e.target.value)}
        >
          {sortOptions.map((option) => (
            <option
              key={sortValue(option.field, option.direction)}
              value={sortValue(option.field, option.direction)}
            >
              {option.label}
            </option>
          ))}
        </select>
        {onNewCard && (
          <button type="button" className="primary-button" onClick={onNewCard}>
            {newCardLabel}
          </button>
        )}
      </div>

      {isLoading && items.length === 0 ? (
        <div className="card-list-empty">
          <p>{loadingText}</p>
        </div>
      ) : error ? (
        <div className="card-list-empty">
          <p>{errorText}</p>
        </div>
      ) : items.length === 0 ? (
        <div className="card-list-empty">
          <p>{emptyTitle}</p>
          {emptyHint && <p>{emptyHint}</p>}
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
            {items.map((card) => (
              <div
                key={card.id}
                className="card-list-row"
                onClick={() => onOpenCard(card)}
              >
                <div className="card-list-thumb">
                  {card.has_image && imageBasePath ? (
                    <img
                      src={cardImageSrc(imageBasePath, card.code, revision)}
                      alt=""
                      loading="lazy"
                      decoding="async"
                    />
                  ) : (
                    <svg width="16" height="20" viewBox="0 0 16 20" fill="none" stroke="currentColor" strokeWidth="1">
                      <rect x="1" y="1" width="14" height="18" rx="1.5" />
                      <rect x="3" y="3" width="10" height="8" rx="0.5" />
                    </svg>
                  )}
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
                    <span key={tag} className={subtypeTagClass(tag, card.primary_type)}>
                      {tag}
                    </span>
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
              {buildPageNumbers(page, totalPages).map((item, idx) =>
                item === "..." ? (
                  <span key={`ellipsis-${idx}`} className="page-ellipsis">...</span>
                ) : (
                  <button
                    key={item}
                    type="button"
                    className={`page-num ${item === page ? "active" : ""}`}
                    onClick={() => setPage(item as number)}
                  >
                    {item}
                  </button>
                ),
              )}
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
