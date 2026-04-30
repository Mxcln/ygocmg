import { useEffect, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { standardPackApi } from "../../shared/api/standardPackApi";
import { jobApi } from "../../shared/api/jobApi";
import { useShellStore } from "../../shared/stores/shellStore";
import { formatError, formatTimestamp } from "../../shared/utils/format";
import { formatJobError, formatJobStage, formatJobStatus } from "../../shared/utils/messages";
import type { GlobalConfig } from "../../shared/contracts/config";
import type { CardListRow } from "../../shared/contracts/card";
import type { JobSnapshot } from "../../shared/contracts/job";
import type { StandardCardSortField } from "../../shared/contracts/standardPack";
import { languageLabel } from "../../shared/utils/language";
import { useAppI18n } from "../../shared/i18n";
import { CardBrowserPanel } from "../card/CardBrowserPanel";
import type { CardBrowserQuery } from "../card/CardBrowserPanel";
import { StringsBrowserPanel } from "../strings/StringsBrowserPanel";
import type { StringsBrowserQuery } from "../strings/StringsBrowserPanel";
import { StandardCardInspector } from "./StandardCardInspector";
import drawerStyles from "../card/CardEditDrawer.module.css";
import shared from "../../shared/styles/shared.module.css";
import styles from "./StandardPackView.module.css";

type StandardTab = "cards" | "strings";

function stateLabel(state: string, td: (id: string, defaultMessage: string) => string): string {
  switch (state) {
    case "ready":
      return td("standard.state.ready", "Ready");
    case "stale":
      return td("standard.state.stale", "Stale");
    case "missing_index":
      return td("standard.state.missingIndex", "Missing Index");
    case "missing_language":
      return td("standard.state.missingLanguage", "Missing Language");
    case "missing_source":
      return td("standard.state.missingSource", "Missing Source");
    case "not_configured":
      return td("standard.state.notConfigured", "Not Configured");
    default:
      return td("standard.state.error", "Error");
  }
}

function isTerminalJob(job: JobSnapshot): boolean {
  return ["succeeded", "failed", "cancelled"].includes(job.status);
}

export function StandardPackView({ config }: { config: GlobalConfig }) {
  const { t, td } = useAppI18n();
  const [activeTab, setActiveTab] = useState<StandardTab>("cards");
  const [selectedCode, setSelectedCode] = useState<number | null>(null);
  const [activeJobId, setActiveJobId] = useState<string | null>(null);
  const [lastJob, setLastJob] = useState<JobSnapshot | null>(null);
  const [rebuildError, setRebuildError] = useState<string | null>(null);
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
  }, [activeJobId, jobQuery.data, queryClient]);

  const status = statusQuery.data ?? null;
  const activeJob = jobQuery.data ?? lastJob;
  const rebuilding = activeJobId !== null;
  const canBrowseCards = Boolean(status?.index_exists && status.source_language);
  const canRebuild = Boolean(status?.configured && config.standard_pack_source_language);
  const standardSortOptions = [
    { field: "code" as const, direction: "asc" as const, label: t("card.sort.codeAsc") },
    { field: "code" as const, direction: "desc" as const, label: t("card.sort.codeDesc") },
    { field: "name" as const, direction: "asc" as const, label: t("card.sort.nameAsc") },
    { field: "name" as const, direction: "desc" as const, label: t("card.sort.nameDesc") },
    { field: "type" as const, direction: "asc" as const, label: td("card.sort.typeAsc", "Type Asc") },
    { field: "type" as const, direction: "desc" as const, label: td("card.sort.typeDesc", "Type Desc") },
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
      <div className={styles.standardPackHeader}>
        <div className={styles.standardPackSummary}>
          <strong>{t("sidebar.standardPack")}</strong>
          <span>
            {status
              ? td("standard.summary", "{state} · {count} cards", {
                  state: stateLabel(status.state, td),
                  count: status.card_count,
                })
              : td("standard.loadingStatus", "Loading status...")}
          </span>
        </div>
        <div className={styles.standardPackActions}>
          {!status?.configured && (
            <button type="button" className={shared.ghostButton} onClick={() => openModal("settings")}>
              {t("action.settings")}
            </button>
          )}
          {status?.state === "missing_language" && (
            <button type="button" className={shared.ghostButton} onClick={() => openModal("settings")}>
              {t("action.settings")}
            </button>
          )}
          <button
            type="button"
            className={shared.primaryButton}
            disabled={rebuilding || !canRebuild}
            onClick={() => void handleRebuild()}
          >
            {rebuilding ? td("standard.rebuilding", "Rebuilding...") : td("standard.rebuildIndex", "Rebuild Index")}
          </button>
        </div>
      </div>

      <div
        className={styles.standardStatusStrip}
        data-status={status?.state ?? "loading"}
      >
        {statusQuery.isLoading ? (
          <span>{td("standard.checkingStatus", "Checking standard pack status...")}</span>
        ) : status ? (
          <>
            <span className={styles.statusPill}>{stateLabel(status.state, td)}</span>
            <span title={status.ygopro_path ?? undefined}>{status.ygopro_path ?? td("standard.ygoproNotConfigured", "YGOPro path is not configured")}</span>
            <span>
              {td("standard.sourcePrefix", "Source:")} {status.source_language
                ? languageLabel(config.text_language_catalog, status.source_language)
                : config.standard_pack_source_language
                  ? languageLabel(config.text_language_catalog, config.standard_pack_source_language)
                  : td("standard.notConfigured", "Not configured")}
            </span>
            {status.cdb_path && <span title={status.cdb_path}>{td("standard.cdbPath", "CDB: {path}", { path: status.cdb_path })}</span>}
            <span>{td("standard.indexedPrefix", "Indexed:")} {formatTimestamp(status.indexed_at)}</span>
            {status.message && <span className={styles.statusMessage}>{status.message}</span>}
          </>
        ) : (
          <span>{td("standard.statusUnavailable", "Standard pack status is unavailable.")}</span>
        )}
      </div>

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
          {td("pack.tabs.cards", "Cards")}
        </button>
        <button
          type="button"
          className={`${shared.tabBtn} ${activeTab === "strings" ? "active" : ""}`}
          onClick={() => setActiveTab("strings")}
        >
          {td("pack.tabs.strings", "Strings")}
        </button>
      </div>

      <div className={shared.tabContent}>
        {activeTab === "cards" ? (
          canBrowseCards ? (
            <CardBrowserPanel
              enabled={canBrowseCards}
              queryKeyBase={["standard-cards"]}
              loadPage={loadStandardPage}
              onOpenCard={handleOpenCard}
              sortOptions={standardSortOptions}
              emptyTitle={td("standard.noCards", "No standard cards found.")}
              emptyHint={td("standard.tryAnotherSearch", "Try another search term.")}
            />
          ) : (
            <div className={shared.cardListEmpty}>
              <p>{td("standard.noIndex", "No standard index yet.")}</p>
              <p>{td("standard.configureToBrowseCards", "Configure YGOPro path and rebuild the index to browse standard cards.")}</p>
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
              emptyTitle={td("standard.noStrings", "No standard strings found.")}
              emptyHint={td("standard.noStringsHint", "strings.conf is missing or no entries match the current filter.")}
            />
          ) : (
            <div className={shared.cardListEmpty}>
              <p>{td("standard.noIndex", "No standard index yet.")}</p>
              <p>{td("standard.configureToBrowseStrings", "Configure YGOPro path and rebuild the index to browse standard strings.")}</p>
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
