// DeepSeek chat completions — OpenAI-compatible message/tool types.
// The frontend agent loop builds these; the backend `llm_chat` command only
// injects the API key and forwards the request. See docs/agent_mvp_design.md §3.

export type ChatRole = "system" | "user" | "assistant" | "tool";

export interface ToolCallFunction {
  name: string;
  /** JSON-encoded string of the arguments object. Must be JSON.parse'd. */
  arguments: string;
}

export interface ToolCall {
  id: string;
  type: "function";
  function: ToolCallFunction;
}

export interface ChatMessage {
  role: ChatRole;
  /** assistant tool-call turns may have null content. */
  content: string | null;
  /** present on assistant messages that call tools. */
  tool_calls?: ToolCall[];
  /** present on tool-result messages, pairs with ToolCall.id. */
  tool_call_id?: string;
  /** optional tool name on tool-result messages. */
  name?: string;
}

export interface ToolDefinition {
  type: "function";
  function: {
    name: string;
    description: string;
    parameters: object; // JSON Schema literal
  };
}

export type FinishReason =
  | "stop"
  | "tool_calls"
  | "length"
  | "content_filter"
  | "insufficient_system_resource";

/** Request body sent to the backend `llm_chat` command (key injected server-side). */
export interface ChatRequestBody {
  model: string;
  messages: ChatMessage[];
  tools?: ToolDefinition[];
  thinking?: { type: "enabled" | "disabled" };
  stream: false;
}

export interface ChatChoice {
  index: number;
  message: ChatMessage;
  finish_reason: FinishReason;
}

export interface ChatCompletionResponse {
  id: string;
  choices: ChatChoice[];
  model: string;
  usage?: {
    prompt_tokens: number;
    completion_tokens: number;
    total_tokens: number;
  };
}
