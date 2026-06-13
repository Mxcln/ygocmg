import { useEffect, useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";
import { useShellStore } from "../../shared/stores/shellStore";
import { useAgentStore } from "../../shared/stores/agentStore";
import type { AgentLanguage } from "../../shared/contracts/config";
import { useAppI18n } from "../../shared/i18n";
import { formatValidationIssue } from "../../shared/utils/format";
import { useAgentLoop } from "./useAgentLoop";
import { MarkdownMessage } from "./MarkdownMessage";
import styles from "./AgentSidebar.module.css";

interface AgentSidebarProps {
  hasApiKey: boolean;
  agentLanguage: AgentLanguage;
  collapsed: boolean;
  width: number;
  onToggleCollapsed: () => void;
  onBeginResize: (event: ReactPointerEvent<HTMLDivElement>) => void;
  onOpenSettings: () => void;
}

export function AgentSidebar({
  hasApiKey,
  agentLanguage,
  collapsed,
  width,
  onToggleCollapsed,
  onBeginResize,
  onOpenSettings,
}: AgentSidebarProps) {
  const { t } = useAppI18n();
  const display = useAgentStore((s) => s.display);
  const status = useAgentStore((s) => s.status);
  const pending = useAgentStore((s) => s.pendingConfirmation);
  const clearConversation = useAgentStore((s) => s.clearConversation);
  const { sendMessage, resolveConfirmation } = useAgentLoop(agentLanguage);
  const [input, setInput] = useState("");
  const scrollRef = useRef<HTMLDivElement>(null);

  const activePackId = useShellStore((s) => s.activePackId);
  const packMetadataMap = useShellStore((s) => s.packMetadataMap);
  const activePackName = (activePackId && packMetadataMap[activePackId]?.name) || null;

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight });
  }, [display, status]);

  if (collapsed) {
    return (
      <aside className={styles.collapsedBar}>
        <button
          type="button"
          className={styles.collapsedToggle}
          onClick={onToggleCollapsed}
          aria-label={t("agent.open")}
          title={t("agent.title")}
        >
          <svg width="18" height="18" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3">
            <path d="M3 3h10v8H6l-3 2.5V11H3z" />
          </svg>
        </button>
      </aside>
    );
  }

  const busy = status !== "idle";

  function handleSubmit() {
    if (busy) return;
    const text = input;
    setInput("");
    void sendMessage(text);
  }

  return (
    <aside className={styles.sidebar} style={{ width }}>
      <div className={styles.resizeHandle} role="separator" aria-orientation="vertical" aria-label={t("agent.resize")} onPointerDown={onBeginResize} />

      <header className={styles.header}>
        <span className={styles.title}>{t("agent.title")}</span>
        <div className={styles.headerActions}>
          <button type="button" className={styles.iconBtn} onClick={clearConversation} title={t("agent.clear")} aria-label={t("agent.clear")}>
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3">
              <path d="M3 4h10M6 4V2.5h4V4M5 4l.5 9h5L11 4" />
            </svg>
          </button>
          <button type="button" className={styles.iconBtn} onClick={onToggleCollapsed} title={t("agent.collapse")} aria-label={t("agent.collapse")}>
            <svg width="14" height="14" viewBox="0 0 10 10" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round">
              <path d="M3 1l4 4-4 4" />
            </svg>
          </button>
        </div>
      </header>

      <div className={styles.contextChip}>
        {activePackName
          ? t("agent.contextPack", { pack: activePackName })
          : t("agent.contextNoPack")}
      </div>

      {!hasApiKey ? (
        <div className={styles.noKey}>
          <p className={styles.noKeyTitle}>{t("agent.noKeyTitle")}</p>
          <p className={styles.noKeyHint}>{t("agent.noKeyHint")}</p>
          <button type="button" className={styles.noKeyAction} onClick={onOpenSettings}>
            {t("agent.noKeyAction")}
          </button>
        </div>
      ) : (
        <>
          <div className={styles.messages} ref={scrollRef}>
            {display.length === 0 && (
              <div className={styles.empty}>
                <p className={styles.emptyTitle}>{t("agent.emptyTitle")}</p>
                <p className={styles.emptyHint}>{t("agent.emptyHint")}</p>
              </div>
            )}
            {display.map((m) => (
              <div key={m.id} className={`${styles.msg} ${styles[m.role] ?? ""}`}>
                {m.role === "tool" ? (
                  <span className={styles.toolLine}>
                    <span className={styles.toolBadge}>{m.toolName}</span>
                    <code className={styles.toolArgs}>{m.text}</code>
                  </span>
                ) : m.role === "assistant" ? (
                  <MarkdownMessage text={m.text} />
                ) : (
                  <span className={styles.msgText}>{m.text}</span>
                )}
              </div>
            ))}

            {pending && (
              <div className={styles.confirm}>
                <p className={styles.confirmTitle}>{t("agent.confirmTitle")}</p>
                <code className={styles.confirmSummary}>{pending.summary}</code>
                {pending.warnings.length > 0 && (
                  <ul className={styles.confirmWarnings}>
                    {pending.warnings.map((w, i) => (
                      <li key={i}>{formatValidationIssue(w)}</li>
                    ))}
                  </ul>
                )}
                <div className={styles.confirmActions}>
                  <button type="button" className={styles.confirmApply} onClick={() => resolveConfirmation(true)}>
                    {t("agent.confirmApply")}
                  </button>
                  <button type="button" className={styles.confirmCancel} onClick={() => resolveConfirmation(false)}>
                    {t("agent.confirmCancel")}
                  </button>
                </div>
              </div>
            )}

            {status === "running" && (
              <div className={`${styles.msg} ${styles.assistant} ${styles.loading}`}>
                <span className={styles.spinner} aria-hidden="true" />
                <span className={styles.msgText}>{t("agent.thinking")}</span>
              </div>
            )}
          </div>

          <div className={styles.composer}>
            <textarea
              className={styles.input}
              value={input}
              placeholder={t("agent.placeholder")}
              rows={2}
              disabled={busy}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  handleSubmit();
                }
              }}
            />
            <button type="button" className={styles.sendBtn} onClick={handleSubmit} disabled={busy || !input.trim()}>
              {t("agent.send")}
            </button>
          </div>
        </>
      )}
    </aside>
  );
}
