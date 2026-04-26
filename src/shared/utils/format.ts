export function normalizeNullablePath(value: string): string | null {
  const trimmed = value.trim();
  return trimmed || null;
}

export function normalizeOptionalText(value: string): string | null {
  const trimmed = value.trim();
  return trimmed || null;
}

export function parseNumberInput(value: string, fallback: number): number {
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? parsed : fallback;
}

export function buildSuggestedWorkspacePath(root: string, workspaceName: string): string {
  const normalizedRoot = root.trim();
  const normalizedName = slugifyWorkspaceName(workspaceName);

  if (!normalizedRoot) return normalizedName;
  if (!normalizedName) return normalizedRoot;

  const separator =
    normalizedRoot.includes("\\") || /^[A-Za-z]:/.test(normalizedRoot) ? "\\" : "/";
  return `${normalizedRoot.replace(/[\\/]+$/, "")}${separator}${normalizedName}`;
}

export function slugifyWorkspaceName(value: string): string {
  return value
    .trim()
    .replace(/[<>:"/\\|?*]+/g, "-")
    .replace(/\s+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "");
}

export function formatTimestamp(value: string | null): string {
  if (!value) return "No recorded time";

  const timestamp = new Date(value);
  if (Number.isNaN(timestamp.getTime())) return value;

  return timestamp.toLocaleString("en-US", { hour12: false });
}

export function formatError(error: unknown): string {
  if (typeof error === "object" && error !== null && "code" in error && "message" in error) {
    const appError = error as { code: string; message: string };
    return `${appError.code}: ${appError.message}`;
  }

  if (error instanceof Error) return error.message;

  return "An unknown error occurred.";
}
