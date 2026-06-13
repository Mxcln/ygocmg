import type { AgentTool } from "./types";
import {
  getCardTool,
  listCardsTool,
  searchStandardCardsTool,
  getConfigTool,
  getPackInfoTool,
  listPacksTool,
  suggestCardCodeTool,
  listSetnamesTool,
} from "./readTools";
import { createCardTool, moveCardsTool, updateCardTool } from "./writeTools";
import {
  switchPackTool,
  openPackTool,
  closePackTool,
  createPackTool,
  updatePackMetaTool,
  deletePackTool,
} from "./packTools";
import { toToolDefinition } from "./types";

export const AGENT_TOOLS: AgentTool[] = [
  listCardsTool,
  getCardTool,
  searchStandardCardsTool,
  getConfigTool,
  getPackInfoTool,
  listPacksTool,
  suggestCardCodeTool,
  listSetnamesTool,
  createCardTool,
  updateCardTool,
  moveCardsTool,
  switchPackTool,
  openPackTool,
  closePackTool,
  createPackTool,
  updatePackMetaTool,
  deletePackTool,
];

const TOOLS_BY_NAME = new Map(AGENT_TOOLS.map((tool) => [tool.name, tool]));

export function getTool(name: string): AgentTool | undefined {
  return TOOLS_BY_NAME.get(name);
}

/** Tool definitions sent in the DeepSeek request body. */
export const TOOL_DEFINITIONS = AGENT_TOOLS.map(toToolDefinition);

export type { AgentTool, ToolContext } from "./types";
