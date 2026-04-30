import { useState } from "react";
import { useQuery, keepPreviousData } from "@tanstack/react-query";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { CardListRow, SortDirection } from "../../shared/contracts/card";
import { useAppI18n } from "../../shared/i18n";
import { formatPrimaryType, formatSubtypeDisplayPart } from "../../shared/utils/cardLabels";
import shared from "../../shared/styles/shared.module.css";
import styles from "./CardBrowserPanel.module.css";

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

function formatStat(val: number | null): string {
  if (val === null) return "";
  if (val === -2) return "?";
  return String(val);
}

function subtypeTagDataFlag(tag: string, primaryType: string): string {
  if (primaryType === "monster") {
    return tag.toLowerCase().replace(/[^a-z]/g, "");
  }
  return primaryType;
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
  newCardLabel,
  sortOptions,
  emptyTitle,
  emptyHint,
  loadingText,
  errorText,
}: CardBrowserPanelProps) {
  const { t } = useAppI18n();
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
  const displaySortOptions: CardBrowserSortOption[] =
    sortOptions ?? [
      { field: "code", direction: "asc", label: t("card.sort.codeAsc") },
      { field: "code", direction: "desc", label: t("card.sort.codeDesc") },
      { field: "name", direction: "asc", label: t("card.sort.nameAsc") },
      { field: "name", direction: "desc", label: t("card.sort.nameDesc") },
    ];

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
      <div className={styles.cardListToolbar}>
        <input
          className={styles.cardSearchInput}
          type="text"
          placeholder={t("card.list.search")}
          value={keyword}
          onChange={(e) => handleSearchChange(e.target.value)}
        />
        <select
          className={styles.cardSortSelect}
          value={sortValue(sortBy, sortDirection)}
          onChange={(e) => handleSortChange(e.target.value)}
        >
          {displaySortOptions.map((option) => (
            <option
              key={sortValue(option.field, option.direction)}
              value={sortValue(option.field, option.direction)}
            >
              {option.label}
            </option>
          ))}
        </select>
        {onNewCard && (
          <button type="button" className={shared.primaryButton} onClick={onNewCard}>
            {newCardLabel ?? t("card.list.newCard")}
          </button>
        )}
      </div>

      {isLoading && items.length === 0 ? (
        <div className={shared.cardListEmpty}>
          <p>{loadingText ?? t("card.list.loading")}</p>
        </div>
      ) : error ? (
        <div className={shared.cardListEmpty}>
          <p>{errorText ?? t("card.list.failed")}</p>
        </div>
      ) : items.length === 0 ? (
        <div className={shared.cardListEmpty}>
          <p>{emptyTitle}</p>
          {emptyHint && <p>{emptyHint}</p>}
        </div>
      ) : (
        <>
          <div className={styles.cardListHeader}>
            <span />
            <span>{t("card.list.code")}</span>
            <span>{t("card.list.name")}</span>
            <span>{t("card.list.type")}</span>
            <span>{t("card.list.subtype")}</span>
            <span>{t("card.list.atk")}</span>
            <span>{t("card.list.def")}</span>
            <span>{t("card.list.levelShort")}</span>
            <span />
          </div>
          <div className={styles.cardListBody}>
            {items.map((card) => (
              <div
                key={card.id}
                className={styles.cardListRow}
                onClick={() => onOpenCard(card)}
              >
                <div className={styles.cardListThumb}>
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
                <span className={styles.cardListCode}>{card.code}</span>
                <span className={styles.cardListName} title={card.name}>
                  {card.name || t("card.noName")}
                </span>
                <span className={styles.cardTypeBadge} data-type={card.primary_type}>
                  {formatPrimaryType(card.primary_type)}
                </span>
                <span className={styles.cardListSubtype}>
                  {card.subtype_display.split(" / ").map((tag) => (
                    <span
                      key={tag}
                      className={styles.subtypeTag}
                      data-flag={subtypeTagDataFlag(tag, card.primary_type)}
                    >
                      {formatSubtypeDisplayPart(tag)}
                    </span>
                  ))}
                </span>
                <span className={styles.cardListStat}>{card.atk !== null ? formatStat(card.atk) : ""}</span>
                <span className={styles.cardListStat}>{card.def !== null ? formatStat(card.def) : ""}</span>
                <span className={styles.cardListStat}>{card.level !== null ? String(card.level) : ""}</span>
                <span className={styles.cardListAssets}>
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
            <div className={shared.cardListPagination}>
              <button
                type="button"
                disabled={page <= 1}
                onClick={() => setPage((p) => Math.max(1, p - 1))}
              >
                {t("card.list.prev")}
              </button>
              {buildPageNumbers(page, totalPages).map((item, idx) =>
                item === "..." ? (
                  <span key={`ellipsis-${idx}`} className={shared.pageEllipsis}>...</span>
                ) : (
                  <button
                    key={item}
                    type="button"
                    className={`${shared.pageNum} ${item === page ? "active" : ""}`}
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
                {t("card.list.next")}
              </button>
            </div>
          )}
        </>
      )}
    </>
  );
}
