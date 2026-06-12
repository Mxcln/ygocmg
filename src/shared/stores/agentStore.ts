import { create } from "zustand";
import type { ChatMessage } from "../contracts/agent";
import type { ValidationIssue } from "../contracts/common";

/** A pending backend confirmation surfaced by a write tool (WriteResult.needs_confirmation). */
export interface PendingConfirmation {
  /** the tool_call id this confirmation belongs to (for resuming the loop). */
  toolCallId: string;
  toolName: string;
  confirmationToken: string;
  warnings: ValidationIssue[];
  preview: unknown | null;
  /** human-readable summary of what the tool was asked to do. */
  summary: string;
}

/** UI-facing transcript entry. Distinct from wire ChatMessage so we can render richly. */
export interface DisplayMessage {
  id: string;
  role: "user" | "assistant" | "tool" | "error" | "system-notice";
  text: string;
  /** for tool entries: the tool name that ran. */
  toolName?: string;
}

export type AgentStatus = "idle" | "running" | "awaiting_confirmation";

interface AgentState {
  /** Wire history sent to DeepSeek every turn (system prompt is prepended by the loop). */
  wireMessages: ChatMessage[];
  /** What the user sees in the sidebar. */
  display: DisplayMessage[];
  status: AgentStatus;
  pendingConfirmation: PendingConfirmation | null;

  setStatus: (status: AgentStatus) => void;
  appendWire: (message: ChatMessage) => void;
  appendWireMany: (messages: ChatMessage[]) => void;
  appendDisplay: (message: DisplayMessage) => void;
  setPendingConfirmation: (pending: PendingConfirmation | null) => void;
  clearConversation: () => void;
}

export const useAgentStore = create<AgentState>()((set) => ({
  wireMessages: [],
  display: [],
  status: "idle",
  pendingConfirmation: null,

  setStatus: (status) => set({ status }),
  appendWire: (message) => set((state) => ({ wireMessages: [...state.wireMessages, message] })),
  appendWireMany: (messages) =>
    set((state) => ({ wireMessages: [...state.wireMessages, ...messages] })),
  appendDisplay: (message) => set((state) => ({ display: [...state.display, message] })),
  setPendingConfirmation: (pending) => set({ pendingConfirmation: pending }),
  clearConversation: () =>
    set({ wireMessages: [], display: [], status: "idle", pendingConfirmation: null }),
}));
