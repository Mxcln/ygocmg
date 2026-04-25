import { invoke } from "@tauri-apps/api/core";

export interface AppError {
  code: string;
  message: string;
  details?: Record<string, unknown>;
}

function isAppError(value: unknown): value is AppError {
  return typeof value === "object" && value !== null && "code" in value && "message" in value;
}

export async function invokeApi<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    if (isAppError(error)) {
      throw error;
    }

    throw {
      code: "frontend.invoke_failed",
      message: error instanceof Error ? error.message : String(error),
    } satisfies AppError;
  }
}
