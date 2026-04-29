import { useEffect, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { standardPackApi } from "../../shared/api/standardPackApi";
import { jobApi } from "../../shared/api/jobApi";
import { useShellStore } from "../../shared/stores/shellStore";
import { formatError, formatTimestamp } from "../../shared/utils/format";
import type { GlobalConfig } from "../../shared/contracts/config";
import type { CardListRow } from "../../shared/contracts/card";
import type { JobSnapshot } from "../../shared/contracts/job";
import type { StandardCardSortField } from "../../shared/contracts/standardPack";
import { languageLabel } from "../../shared/utils/language";
import { CardBrowserPanel } from "../card/CardBrowserPanel";
import type { CardBrowserQuery } from "../card/CardBrowserPanel";
import { StringsBrowserPanel } from "../strings/StringsBrowserPanel";
import type { StringsBrowserQuery } from "../strings/StringsBrowserPanel";
import { StandardCardInspector } from "./StandardCardInspector";
import drawerStyles from "../card/CardEditDrawer.module.css";
import shared from "../../shared/styles/shared.module.css";
import styles from "./StandardPackView.module.css";

type StandardTab = "cards" | "strings";

const STANDARD_SORT_OPTIONS = [
  { field: "code" as const, direction: "asc" as const, label: "Code Asc" },
  { field: "code" as const, direction: "desc" as const, label: "Code Desc" },
  { field: "name" as const, direction: "asc" as const, label: "Name A-Z" },
  { field: "name" as const, direction: "desc" as const, label: "Name Z-A" },
  { field: "type" as const, direction: "asc" as const, label: "Type Asc" },
  { field: "type" as const, direction: "desc" as const, label: "Type Desc" },
];

function stateLabel(state: string): string {
  switch (state) {
    case "ready":
      return "Ready";
    case "stale":
      return "Stale";
    case "missing_index":
      return "Missing Index";
    case "missing_language":
      return "Missing Language";
    case "missing_source":
      return "Missing Source";
    case "not_configured":
      return "Not Configured";
    default:
      return "Error";
  }
}

function isTerminalJob(job: JobSnapshot): boolean {
  return ["succeeded", "failed", "cancelled"].includes(job.status);
}

export function StandardPackView({ config }: { config: GlobalConfig }) {
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
          <strong>Standard Pack</strong>
          <span>
            {status
              ? `${stateLabel(status.state)} · ${status.card_count} cards`
              : "Loading status..."}
          </span>
        </div>
        <div className={styles.standardPackActions}>
          {!status?.configured && (
            <button type="button" className={shared.ghostButton} onClick={() => openModal("settings")}>
              Settings
            </button>
          )}
          {status?.state === "missing_language" && (
            <button type="button" className={shared.ghostButton} onClick={() => openModal("settings")}>
              Settings
            </button>
          )}
          <button
            type="button"
            className={shared.primaryButton}
            disabled={rebuilding || !canRebuild}
            onClick={() => void handleRebuild()}
          >
            {rebuilding ? "Rebuilding..." : "Rebuild Index"}
          </button>
        </div>
      </div>

      <div
        className={styles.standardStatusStrip}
        data-status={status?.state ?? "loading"}
      >
        {statusQuery.isLoading ? (
          <span>Checking standard pack status...</span>
        ) : status ? (
          <>
            <span className={styles.statusPill}>{stateLabel(status.state)}</span>
            <span title={status.ygopro_path ?? undefined}>{status.ygopro_path ?? "YGOPro path is not configured"}</span>
            <span>
              Source: {status.source_language
                ? languageLabel(config.text_language_catalog, status.source_language)
                : config.standard_pack_source_language
                  ? languageLabel(config.text_language_catalog, config.standard_pack_source_language)
                  : "Not configured"}
            </span>
            {status.cdb_path && <span title={status.cdb_path}>CDB: {status.cdb_path}</span>}
            <span>Indexed: {formatTimestamp(status.indexed_at)}</span>
            {status.message && <span className={styles.statusMessage}>{status.message}</span>}
          </>
        ) : (
          <span>Standard pack status is unavailable.</span>
        )}
      </div>

      {activeJob && (
        <div className={styles.standardJobStrip} data-status={activeJob.status}>
          <span>{activeJob.status}</span>
          <strong>{activeJob.stage}</strong>
          <span>{activeJob.progress_percent ?? 0}%</span>
          {activeJob.message && <span>{activeJob.message}</span>}
          {activeJob.error && <span>{activeJob.error.code}: {activeJob.error.message}</span>}
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
          Cards
        </button>
        <button
          type="button"
          className={`${shared.tabBtn} ${activeTab === "strings" ? "active" : ""}`}
          onClick={() => setActiveTab("strings")}
        >
          Strings
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
              sortOptions={STANDARD_SORT_OPTIONS}
              emptyTitle="No standard cards found."
              emptyHint="Try another search term."
            />
          ) : (
            <div className={shared.cardListEmpty}>
              <p>No standard index yet.</p>
              <p>Configure YGOPro path and rebuild the index to browse standard cards.</p>
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
              emptyTitle="No standard strings found."
              emptyHint="strings.conf is missing or no entries match the current filter."
            />
          ) : (
            <div className={shared.cardListEmpty}>
              <p>No standard index yet.</p>
              <p>Configure YGOPro path and rebuild the index to browse standard strings.</p>
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
