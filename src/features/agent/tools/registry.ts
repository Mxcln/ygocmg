import type { AgentTool } from "./types";
import { getCardTool, listCardsTool, searchStandardCardsTool } from "./readTools";
import { createCardTool, moveCardsTool, updateCardTool } from "./writeTools";
import { toToolDefinition } from "./types";

export const AGENT_TOOLS: AgentTool[] = [
  listCardsTool,
  getCardTool,
  searchStandardCardsTool,
  createCardTool,
  updateCardTool,
  moveCardsTool,
];

const TOOLS_BY_NAME = new Map(AGENT_TOOLS.map((tool) => [tool.name, tool]));

export function getTool(name: string): AgentTool | undefined {
  return TOOLS_BY_NAME.get(name);
}

/** Tool definitions sent in the DeepSeek request body. */
export const TOOL_DEFINITIONS = AGENT_TOOLS.map(toToolDefinition);

export type { AgentTool, ToolContext } from "./types";
