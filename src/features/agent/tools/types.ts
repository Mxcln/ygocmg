import type { ToolDefinition } from "../../../shared/contracts/agent";

/**
 * Context injected by the agent loop into every tool execution: the current
 * workspace/pack snapshot from the shell, so the model doesn't have to supply
 * these ids itself. See docs/agent_mvp_design.md §5, §6.
 */
export interface ToolContext {
  workspaceId: string | null;
  packId: string | null;
}

export interface AgentTool {
  name: string;
  description: string;
  /** JSON Schema literal for the tool's parameters. */
  parameters: object;
  /** read-only tools auto-execute; write tools may hit the backend confirmation gate. */
  readOnly: boolean;
  /** Calls the corresponding src/shared/api wrapper. Returns any JSON-serializable result. */
  execute(args: Record<string, unknown>, ctx: ToolContext): Promise<unknown>;
}

/** Build the OpenAI/DeepSeek tool definition sent in the request body. */
export function toToolDefinition(tool: AgentTool): ToolDefinition {
  return {
    type: "function",
    function: {
      name: tool.name,
      description: tool.description,
      parameters: tool.parameters,
    },
  };
}

export class ToolError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ToolError";
  }
}

/** Throws if the shell has no active pack — most tools need one. */
export function requirePack(ctx: ToolContext): { workspaceId: string; packId: string } {
  if (!ctx.workspaceId || !ctx.packId) {
    throw new ToolError(
      "No active pack. Ask the user to open a pack before running pack operations.",
    );
  }
  return { workspaceId: ctx.workspaceId, packId: ctx.packId };
}
