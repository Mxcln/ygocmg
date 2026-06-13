import { cardApi } from "../../shared/api/cardApi";
import type { WriteResult } from "../../shared/contracts/card";
import type {
  ChatMessage,
  ChatRequestBody,
  ToolCall,
} from "../../shared/contracts/agent";
import { agentApi } from "../../shared/api/agentApi";
import { queryClient } from "../../app/providers";
import { getTool, TOOL_DEFINITIONS } from "./tools/registry";
import type { ToolContext } from "./tools/types";
import { buildSystemPrompt } from "./systemPrompt";

const MODEL = "deepseek-v4-flash";
const MAX_ROUNDS = 8;

export interface ConfirmationRequest {
  toolCallId: string;
  toolName: string;
  confirmationToken: string | null;
  warnings: WriteResult<unknown> extends { warnings: infer W } ? W : never;
  preview: unknown;
  summary: string;
}

/** Callbacks the loop uses to talk to the store/UI. */
export interface LoopHooks {
  /** Append a wire message to history (persisted in store). */
  pushWire: (message: ChatMessage) => void;
  /** Show a human-facing line in the transcript. */
  showAssistant: (text: string) => void;
  showToolRun: (toolName: string, summary: string) => void;
  showError: (text: string) => void;
  showNotice: (text: string) => void;
  /**
   * Ask the UI to confirm a pending backend write. Resolves true (apply) or false (cancel).
   * The loop awaits this; the UI drives resolution.
   */
  requestConfirmation: (req: ConfirmationRequest) => Promise<boolean>;
}

function isWriteResult(value: unknown): value is WriteResult<unknown> {
  return (
    typeof value === "object" &&
    value !== null &&
    "status" in value &&
    ((value as { status: unknown }).status === "ok" ||
      (value as { status: unknown }).status === "needs_confirmation")
  );
}

interface CommandConfirmation {
  status: "needs_confirmation";
  confirmation: { summary: string; commit: () => Promise<unknown> };
}

function isCommandConfirmation(value: unknown): value is CommandConfirmation {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  const conf = v.confirmation as Record<string, unknown> | null | undefined;
  return (
    v.status === "needs_confirmation" &&
    typeof conf === "object" &&
    conf !== null &&
    typeof conf.commit === "function"
  );
}

/** Build the per-turn context block (current shell snapshot). */
export function buildContextBlock(snapshot: {
  workspaceName: string | null;
  activePackId: string | null;
  activePackName: string | null;
  activeView: string;
  openPackNames: string[];
  selectedCard: { id: string; name: string } | null;
  checkedCards: { id: string; name: string }[];
}): string {
  const lines = [
    "[Current state]",
    `Workspace: ${snapshot.workspaceName ?? "(none)"}`,
    `Active pack: ${snapshot.activePackName ?? snapshot.activePackId ?? "(none)"}`,
    `Current view: ${snapshot.activeView}`,
    `Open packs: ${snapshot.openPackNames.length ? snapshot.openPackNames.join(", ") : "(none)"}`,
    `Selected card: ${
      snapshot.selectedCard
        ? `${snapshot.selectedCard.name || "(unnamed)"} (id=${snapshot.selectedCard.id})`
        : "(none)"
    }`,
    `Checked cards: ${
      snapshot.checkedCards.length
        ? `${snapshot.checkedCards.length} selected: ${snapshot.checkedCards
            .map((c) => c.name || `(id=${c.id})`)
            .join(", ")}`
        : "(none)"
    }`,
  ];
  return lines.join("\n");
}

/** Execute a single tool call; returns the tool-result content string for the wire. */
async function runToolCall(
  call: ToolCall,
  ctx: ToolContext,
  hooks: LoopHooks,
): Promise<string> {
  const tool = getTool(call.function.name);
  if (!tool) {
    return JSON.stringify({ error: `Unknown tool: ${call.function.name}` });
  }

  let args: Record<string, unknown>;
  try {
    args = call.function.arguments ? JSON.parse(call.function.arguments) : {};
  } catch {
    return JSON.stringify({
      error: `Invalid JSON arguments for ${call.function.name}: ${call.function.arguments}`,
    });
  }

  try {
    hooks.showToolRun(tool.name, summarizeArgs(tool.name, args));
    const result = await tool.execute(args, ctx);

    // Command-layer confirmation (delete_pack): commit-closure model.
    if (!tool.readOnly && isCommandConfirmation(result)) {
      return handleCommandConfirmation(result, call, hooks);
    }
    // Card write tools and pack command ok-results both shaped {status, data}.
    if (!tool.readOnly && isWriteResult(result)) {
      return handleWriteResult(result, call, args, hooks);
    }
    return JSON.stringify(result ?? null);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return JSON.stringify({ error: message });
  }
}

/**
 * Refresh card caches after an agent card write. UI editors invalidate these
 * keys themselves; agent writes go through this loop, so we mirror it here.
 * `["cards"]` matches every paged list query; `["card"]` matches every single
 * card detail query (both via React Query prefix matching).
 */
function invalidateCardCaches(): void {
  void queryClient.invalidateQueries({ queryKey: ["cards"] });
  void queryClient.invalidateQueries({ queryKey: ["card"] });
}

async function handleWriteResult(
  result: WriteResult<unknown>,
  call: ToolCall,
  args: Record<string, unknown>,
  hooks: LoopHooks,
): Promise<string> {
  if (result.status === "ok") {
    invalidateCardCaches();
    return JSON.stringify({ status: "ok", data: result.data });
  }

  // needs_confirmation: pause and ask the UI.
  const apply = await hooks.requestConfirmation({
    toolCallId: call.id,
    toolName: call.function.name,
    confirmationToken: result.confirmation_token,
    warnings: result.warnings,
    preview: result.preview,
    summary: summarizeArgs(call.function.name, args),
  });

  if (!apply) {
    return JSON.stringify({ status: "cancelled_by_user" });
  }

  try {
    const confirmed = await cardApi.confirmCardWrite({
      confirmationToken: result.confirmation_token,
    });
    invalidateCardCaches();
    return JSON.stringify({ status: "ok", data: confirmed });
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return JSON.stringify({ error: `Confirmation failed: ${message}` });
  }
}

async function handleCommandConfirmation(
  result: CommandConfirmation,
  call: ToolCall,
  hooks: LoopHooks,
): Promise<string> {
  const apply = await hooks.requestConfirmation({
    toolCallId: call.id,
    toolName: call.function.name,
    confirmationToken: null,
    warnings: [],
    preview: null,
    summary: result.confirmation.summary,
  });
  if (!apply) {
    return JSON.stringify({ status: "cancelled_by_user" });
  }
  try {
    const data = await result.confirmation.commit();
    return JSON.stringify({ status: "ok", data: data ?? null });
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return JSON.stringify({ error: `Operation failed: ${message}` });
  }
}

function summarizeArgs(toolName: string, args: Record<string, unknown>): string {
  const parts = Object.entries(args)
    .map(([k, v]) => `${k}=${typeof v === "object" ? JSON.stringify(v) : String(v)}`)
    .join(", ");
  return parts ? `${toolName}(${parts})` : `${toolName}()`;
}

/**
 * Run one agent turn: send history + tools, execute any tool calls, loop until
 * the model returns a final text answer or MAX_ROUNDS is hit. Mutates wire
 * history via hooks.pushWire.
 */
export async function runAgentTurn(
  history: ChatMessage[],
  contextBlock: string,
  ctx: ToolContext,
  hooks: LoopHooks,
  locale: string,
): Promise<void> {
  // The first user message of this turn gets the context block prepended.
  const working = [...history];
  const systemPrompt = buildSystemPrompt(locale);

  for (let round = 0; round < MAX_ROUNDS; round++) {
    const body: ChatRequestBody = {
      model: MODEL,
      thinking: { type: "disabled" },
      messages: [
        { role: "system", content: `${systemPrompt}\n\n${contextBlock}` },
        ...working,
      ],
      tools: TOOL_DEFINITIONS,
      stream: false,
    };

    const response = await agentApi.chat(body);
    const choice = response.choices?.[0];
    if (!choice) {
      hooks.showError("Empty response from DeepSeek.");
      return;
    }

    const assistantMsg = choice.message;
    working.push(assistantMsg);
    hooks.pushWire(assistantMsg);

    if (choice.finish_reason === "tool_calls" && assistantMsg.tool_calls?.length) {
      // Execute tool calls serially (write confirmations one at a time).
      for (const call of assistantMsg.tool_calls) {
        const content = await runToolCall(call, ctx, hooks);
        const toolMsg: ChatMessage = {
          role: "tool",
          tool_call_id: call.id,
          content,
        };
        working.push(toolMsg);
        hooks.pushWire(toolMsg);
      }
      continue; // re-send with tool results
    }

    // finish_reason === "stop" (or anything non-tool): final answer.
    if (assistantMsg.content) {
      hooks.showAssistant(assistantMsg.content);
    }
    return;
  }

  hooks.showNotice(
    `Reached the ${MAX_ROUNDS}-round tool limit. Stopping to avoid looping. Ask me to continue if needed.`,
  );
}
