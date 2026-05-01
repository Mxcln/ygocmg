import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { standardPackApi } from "../../shared/api/standardPackApi";
import { jobApi } from "../../shared/api/jobApi";
import { useShellStore } from "../../shared/stores/shellStore";
import { formatError, formatTimestamp } from "../../shared/utils/format";
import { formatJobError, formatJobStage, formatJobStatus } from "../../shared/utils/messages";
import type { GlobalConfig } from "../../shared/contracts/config";
import type { CardListRow } from "../../shared/contracts/card";
import type { JobSnapshot } from "../../shared/contracts/job";
import type { StandardCardSearchFilters, StandardCardSortField } from "../../shared/contracts/standardPack";
import { languageLabel } from "../../shared/utils/language";
import type { PrimitiveType } from "react-intl";
import { useAppI18n, type AppMessageId } from "../../shared/i18n";
import { CardBrowserPanel } from "../card/CardBrowserPanel";
import type { CardBrowserQuery } from "../card/CardBrowserPanel";
import { StringsBrowserPanel } from "../strings/StringsBrowserPanel";
import type { StringsBrowserQuery } from "../strings/StringsBrowserPanel";
import { StandardCardInspector } from "./StandardCardInspector";
import {
  StandardCardAdvancedSearchPanel,
  countStandardCardFilters,
  standardCardFiltersKey,
} from "./StandardCardAdvancedSearchPanel";
import drawerStyles from "../card/CardEditDrawer.module.css";
import shared from "../../shared/styles/shared.module.css";
import styles from "./StandardPackView.module.css";

type StandardTab = "cards" | "strings";

function stateLabel(
  state: string,
  t: (id: AppMessageId, values?: Record<string, PrimitiveType>) => string,
): string {
  switch (state) {
    case "ready":
      return t("standard.state.ready");
    case "stale":
      return t("standard.state.stale");
    case "missing_index":
      return t("standard.state.missingIndex");
    case "missing_language":
      return t("standard.state.missingLanguage");
    case "missing_source":
      return t("standard.state.missingSource");
    case "not_configured":
      return t("standard.state.notConfigured");
    default:
      return t("standard.state.error");
  }
}

function isTerminalJob(job: JobSnapshot): boolean {
  return ["succeeded", "failed", "cancelled"].includes(job.status);
}

export function StandardPackView({ config }: { config: GlobalConfig }) {
  const { t } = useAppI18n();
  const [activeTab, setActiveTab] = useState<StandardTab>("cards");
  const [selectedCode, setSelectedCode] = useState<number | null>(null);
  const [activeJobId, setActiveJobId] = useState<string | null>(null);
  const [lastJob, setLastJob] = useState<JobSnapshot | null>(null);
  const [rebuildError, setRebuildError] = useState<string | null>(null);
  const [metaExpanded, setMetaExpanded] = useState(false);
  const metaBarRef = useRef<HTMLDivElement | null>(null);
  const [metaBarHeight, setMetaBarHeight] = useState(0);
  const [advancedSearchOpen, setAdvancedSearchOpen] = useState(false);
  const [advancedFilters, setAdvancedFilters] = useState<StandardCardSearchFilters | null>(null);
  const queryClient = useQueryClient();
  const openModal = useShellStore((s) => s.openModal);

  const statusQuery = useQuery({
    queryKey: ["standard-pack-status"],
    queryFn: () => standardPackApi.getStatus(),
  });

  const jobQuery = useQuery({
    queryKey: ["standard-pack-job", activeJobId],
    queryFn: () => jobApi.getJobStatus({ jobId: activeJobId! }),
    enabled: activeJobId !== null,
    refetchInterval: activeJobId ? 700 : false,
  });

  useEffect(() => {
    if (!activeJobId || !jobQuery.data || !isTerminalJob(jobQuery.data)) return;
    setLastJob(jobQuery.data);
    setActiveJobId(null);
    void queryClient.invalidateQueries({ queryKey: ["standard-pack-status"] });
    void queryClient.invalidateQueries({ queryKey: ["standard-cards"] });
    void queryClient.invalidateQueries({ queryKey: ["standard-strings"] });
    void queryClient.invalidateQueries({ queryKey: ["standard-card"] });
    void queryClient.invalidateQueries({ queryKey: ["standard-setnames"] });
  }, [activeJobId, jobQuery.data, queryClient]);

  useLayoutEffect(() => {
    const el = metaBarRef.current;
    if (!el) return;

    const measure = () => setMetaBarHeight(el.getBoundingClientRect().height);
    measure();

    const w = globalThis as unknown as Window;

    if (!("ResizeObserver" in w)) {
      w.addEventListener("resize", measure);
      return () => w.removeEventListener("resize", measure);
    }

    const ro = new ResizeObserver(() => measure());
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const status = statusQuery.data ?? null;
  const activeJob = jobQuery.data ?? lastJob;
  const rebuilding = activeJobId !== null;
  const canBrowseCards = Boolean(status?.index_exists && status.source_language);
  const canRebuild = Boolean(status?.configured && config.standard_pack_source_language);
  const activeFilterCount = countStandardCardFilters(advancedFilters);
  const filterKey = useMemo(() => standardCardFiltersKey(advancedFilters), [advancedFilters]);
  const standardSetnamesQuery = useQuery({
    queryKey: ["standard-setnames", status?.source_language ?? null],
    queryFn: () => standardPackApi.listSetnames({ language: status?.source_language ?? null }),
    enabled: advancedSearchOpen && Boolean(status?.source_language),
    staleTime: 5 * 60 * 1000,
  });
  const standardSetnameEntries = useMemo(
    () =>
      standardSetnamesQuery.data?.map((entry) => ({
        key: entry.key,
        name: entry.value,
        source: "standard" as const,
      })) ?? [],
    [standardSetnamesQuery.data],
  );
  const standardSortOptions = [
    { field: "code" as const, direction: "asc" as const, label: t("card.sort.codeAsc") },
    { field: "code" as const, direction: "desc" as const, label: t("card.sort.codeDesc") },
    { field: "name" as const, direction: "asc" as const, label: t("card.sort.nameAsc") },
    { field: "name" as const, direction: "desc" as const, label: t("card.sort.nameDesc") },
    { field: "type" as const, direction: "asc" as const, label: t("card.sort.typeAsc") },
    { field: "type" as const, direction: "desc" as const, label: t("card.sort.typeDesc") },
  ];

  async function handleRebuild() {
    setRebuildError(null);
    setLastJob(null);
    try {
      const job = await standardPackApi.rebuildIndex();
      setActiveJobId(job.job_id);
    } catch (err) {
      setRebuildError(formatError(err));
    }
  }

  async function loadStandardPage(query: CardBrowserQuery) {
    const page = await standardPackApi.searchCards({
      keyword: query.keyword || null,
      filters: advancedFilters,
      sortBy: query.sortBy as StandardCardSortField,
      sortDirection: query.sortDirection,
      page: query.page,
      pageSize: query.pageSize,
    });

    return {
      items: page.items,
      page: page.page,
      page_size: page.page_size,
      total: page.total,
      image_base_path: page.ygopro_path,
      revision: page.revision,
    };
  }

  async function loadStandardStringsPage(query: StringsBrowserQuery) {
    return standardPackApi.searchStrings({
      kindFilter: query.kindFilter || null,
      keyFilter: query.keyFilter,
      keyword: query.keyword || null,
      sortBy: "kind",
      sortDirection: "asc",
      page: query.page,
      pageSize: query.pageSize,
    });
  }

  function handleOpenCard(card: CardListRow) {
    setSelectedCode(card.code);
  }

  return (
    <div className={styles.standardView}>
      <div ref={metaBarRef} className={styles.metaBar}>
        <div className={styles.metaSummary}>
          <strong className={styles.metaTitle}>{t("sidebar.standardPack")}</strong>
          <span className={styles.metaDetail}>
            {status
              ? t("standard.summary", {
                  state: stateLabel(status.state, t),
                  count: status.card_count,
                })
              : t("standard.loadingStatus")}
          </span>
        </div>
        <button
          type="button"
          className={styles.metaToggle}
          onClick={() => setMetaExpanded((open) => !open)}
          aria-label={metaExpanded ? t("pack.metadata.collapse") : t("pack.metadata.expand")}
        >
          <svg
            width="12"
            height="12"
            viewBox="0 0 12 12"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            style={{ transform: metaExpanded ? "rotate(180deg)" : "none", transition: "transform 150ms" }}
          >
            <path d="M2 4l4 4 4-4" />
          </svg>
        </button>
      </div>

      {metaExpanded && (
        <>
          <div
            className={styles.drawerBackdrop}
            style={{ top: metaBarHeight }}
            onClick={() => setMetaExpanded(false)}
          />
          <div className={styles.metaExpanded} style={{ top: metaBarHeight }}>
            <div className={styles.metaGrid}>
              <div className={styles.metaField}>
                <span className={styles.metaFieldLabel}>{t("standard.statePrefix")}</span>
                <span className={styles.metaFieldValue}>
                  {status ? stateLabel(status.state, t) : t("standard.loadingStatus")}
                </span>
              </div>
              <div className={styles.metaField}>
                <span className={styles.metaFieldLabel}>{t("standard.sourcePrefix")}</span>
                <span className={styles.metaFieldValue} title={status?.source_language ?? undefined}>
                  {status?.source_language
                    ? languageLabel(config.text_language_catalog, status.source_language)
                    : config.standard_pack_source_language
                      ? languageLabel(config.text_language_catalog, config.standard_pack_source_language)
                      : t("standard.notConfigured")}
                </span>
              </div>
              <div className={styles.metaField}>
                <span className={styles.metaFieldLabel}>{t("standard.indexedPrefix")}</span>
                <span className={styles.metaFieldValue}>
                  {status?.indexed_at ? formatTimestamp(status.indexed_at) : t("common.none")}
                </span>
              </div>

              <div className={styles.metaFieldRow}>
                <div className={styles.metaFieldStack}>
                  <span className={styles.metaFieldLabel}>{t("standard.ygoproPathLabel")}</span>
                  <span className={styles.metaFieldValueInline} title={status?.ygopro_path ?? undefined}>
                    {status?.ygopro_path ?? t("standard.ygoproNotConfigured")}
                  </span>
                </div>
                {status?.cdb_path ? (
                  <div className={styles.metaFieldStack}>
                    <span className={styles.metaFieldLabel}>CDB</span>
                    <span className={styles.metaFieldValueInline} title={status.cdb_path}>
                      {status.cdb_path}
                    </span>
                  </div>
                ) : (
                  <div className={styles.metaFieldStack} />
                )}
              </div>
              {status?.message && (
                <div className={styles.metaFieldWide}>
                  <span className={styles.metaFieldLabel}>{t("standard.statusMessageLabel")}</span>
                  <span className={styles.metaFieldValueDescription}>{status.message}</span>
                </div>
              )}
            </div>

            <div className={styles.metaActions}>
              {(!status?.configured || status?.state === "missing_language") && (
                <button
                  type="button"
                  className={shared.ghostButton}
                  onClick={() => openModal("settings")}
                >
                  {t("action.settings")}
                </button>
              )}
              <button
                type="button"
                className={`${shared.primaryButton} ${styles.rebuildButton}`}
                disabled={rebuilding || !canRebuild}
                onClick={() => void handleRebuild()}
              >
                {rebuilding ? t("standard.rebuilding") : t("standard.rebuildIndex")}
              </button>
            </div>
          </div>
        </>
      )}

      {activeJob && (
        <div className={styles.standardJobStrip} data-status={activeJob.status}>
          <span>{formatJobStatus(activeJob.status)}</span>
          <strong>{formatJobStage(activeJob.stage)}</strong>
          <span>{activeJob.progress_percent ?? 0}%</span>
          {formatJobError(activeJob) && <span>{formatJobError(activeJob)}</span>}
        </div>
      )}

      {rebuildError && (
        <div className={drawerStyles.cardEditError}>{rebuildError}</div>
      )}

      <div className={shared.tabStrip}>
        <button
          type="button"
          className={`${shared.tabBtn} ${activeTab === "cards" ? "active" : ""}`}
          onClick={() => setActiveTab("cards")}
        >
          {t("pack.tabs.cards")}
        </button>
        <button
          type="button"
          className={`${shared.tabBtn} ${activeTab === "strings" ? "active" : ""}`}
          onClick={() => setActiveTab("strings")}
        >
          {t("pack.tabs.strings")}
        </button>
      </div>

      <div className={shared.tabContent}>
        {activeTab === "cards" ? (
          canBrowseCards ? (
            <CardBrowserPanel
              enabled={canBrowseCards}
              queryKeyBase={["standard-cards"]}
              queryKeyExtra={[filterKey]}
              resetKey={filterKey}
              loadPage={loadStandardPage}
              onOpenCard={handleOpenCard}
              sortOptions={standardSortOptions}
              emptyTitle={t("standard.noCards")}
              emptyHint={activeFilterCount > 0 ? t("standard.tryAnotherFilter") : t("standard.tryAnotherSearch")}
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
                <StandardCardAdvancedSearchPanel
                  open={advancedSearchOpen}
                  filters={advancedFilters}
                  setnameEntries={standardSetnameEntries}
                  onChange={setAdvancedFilters}
                  onClose={() => setAdvancedSearchOpen(false)}
                />
              }
            />
          ) : (
            <div className={shared.cardListEmpty}>
              <p>{t("standard.noIndex")}</p>
              <p>{t("standard.configureToBrowseCards")}</p>
            </div>
          )
        ) : (
          canBrowseCards ? (
            <StringsBrowserPanel
              enabled={canBrowseCards}
              queryKeyBase={["standard-strings"]}
              languages={status?.source_language ? [status.source_language] : []}
              catalog={config.text_language_catalog}
              loadPage={loadStandardStringsPage}
              emptyTitle={t("standard.noStrings")}
              emptyHint={t("standard.noStringsHint")}
            />
          ) : (
            <div className={shared.cardListEmpty}>
              <p>{t("standard.noIndex")}</p>
              <p>{t("standard.configureToBrowseStrings")}</p>
            </div>
          )
        )}
      </div>

      {selectedCode !== null && (
        <StandardCardInspector code={selectedCode} onClose={() => setSelectedCode(null)} />
      )}
    </div>
  );
}
