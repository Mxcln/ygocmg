import { useCallback, useRef } from "react";
import { useShellStore } from "../../shared/stores/shellStore";
import { useAgentStore } from "../../shared/stores/agentStore";
import { useAppI18n } from "../../shared/i18n";
import type { ChatMessage } from "../../shared/contracts/agent";
import type { AgentLanguage } from "../../shared/contracts/config";
import { formatError } from "../../shared/utils/format";
import {
  buildContextBlock,
  runAgentTurn,
  type ConfirmationRequest,
  type LoopHooks,
} from "./agentLoop";
import type { ToolContext } from "./tools/registry";

let messageCounter = 0;
function nextId(): string {
  messageCounter += 1;
  return `m${messageCounter}`;
}

export function useAgentLoop(agentLanguage: AgentLanguage) {
  const store = useAgentStore;
  const { locale } = useAppI18n();
  // The agent replies in the configured language; "auto" follows the app UI language.
  const replyLocale = agentLanguage === "auto" ? locale : agentLanguage;
  // The confirmation gate: the loop awaits this promise; the UI resolves it.
  const confirmResolverRef = useRef<((apply: boolean) => void) | null>(null);

  const resolveConfirmation = useCallback((apply: boolean) => {
    const resolver = confirmResolverRef.current;
    confirmResolverRef.current = null;
    useAgentStore.getState().setPendingConfirmation(null);
    useAgentStore.getState().setStatus("running");
    if (resolver) resolver(apply);
  }, []);

  const sendMessage = useCallback(async (text: string) => {
    const trimmed = text.trim();
    if (!trimmed) return;
    const state = store.getState();
    if (state.status !== "idle") return;

    const shell = useShellStore.getState();
    const ctx: ToolContext = {
      workspaceId: shell.workspaceId,
      packId: shell.activePackId,
    };
    const activePackName =
      (shell.activePackId && shell.packMetadataMap[shell.activePackId]?.name) || null;
    const openPackNames = shell.openPackIds
      .map((id) => shell.packMetadataMap[id])
      .filter((meta): meta is NonNullable<typeof meta> => !!meta && meta.kind === "custom")
      .map((meta) => meta.name);
    const contextBlock = buildContextBlock({
      workspaceName: shell.workspaceName,
      activePackId: shell.activePackId,
      activePackName,
      activeView: shell.activeView?.type ?? "(none)",
      openPackNames,
      selectedCard: shell.selectedCard,
      checkedCards: shell.checkedCards,
    });

    const userMsg: ChatMessage = { role: "user", content: trimmed };
    state.appendWire(userMsg);
    state.appendDisplay({ id: nextId(), role: "user", text: trimmed });
    state.setStatus("running");

    const hooks: LoopHooks = {
      pushWire: (m) => useAgentStore.getState().appendWire(m),
      showAssistant: (t) =>
        useAgentStore.getState().appendDisplay({ id: nextId(), role: "assistant", text: t }),
      showToolRun: (toolName, summary) =>
        useAgentStore
          .getState()
          .appendDisplay({ id: nextId(), role: "tool", text: summary, toolName }),
      showError: (t) =>
        useAgentStore.getState().appendDisplay({ id: nextId(), role: "error", text: t }),
      showNotice: (t) =>
        useAgentStore
          .getState()
          .appendDisplay({ id: nextId(), role: "system-notice", text: t }),
      requestConfirmation: (req: ConfirmationRequest) =>
        new Promise<boolean>((resolve) => {
          confirmResolverRef.current = resolve;
          useAgentStore.getState().setStatus("awaiting_confirmation");
          useAgentStore.getState().setPendingConfirmation({
            toolCallId: req.toolCallId,
            toolName: req.toolName,
            confirmationToken: req.confirmationToken,
            warnings: req.warnings,
            preview: req.preview,
            summary: req.summary,
          });
        }),
    };

    try {
      const history = useAgentStore.getState().wireMessages;
      await runAgentTurn(history, contextBlock, ctx, hooks, replyLocale);
    } catch (err) {
      hooks.showError(formatError(err));
    } finally {
      useAgentStore.getState().setStatus("idle");
      confirmResolverRef.current = null;
    }
  }, [store, replyLocale]);

  return { sendMessage, resolveConfirmation };
}
