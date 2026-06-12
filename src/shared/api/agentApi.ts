import { invokeApi } from "./invoke";
import type { ChatCompletionResponse, ChatRequestBody } from "../contracts/agent";

export const agentApi = {
  /**
   * Forward a DeepSeek chat completion request through the backend, which
   * injects the API key. The body is built by the agent loop (model, messages,
   * tools, thinking, stream:false). Returns the full response JSON.
   */
  chat(body: ChatRequestBody) {
    return invokeApi<ChatCompletionResponse>("llm_chat", { body });
  },
};
