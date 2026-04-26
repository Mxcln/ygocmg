import type { ValidationIssue } from "../contracts/common";

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

function formatIssueParams(params: Record<string, unknown>): string {
  const entries = Object.entries(params);
  if (entries.length === 0) return "";

  return entries
    .map(([key, value]) => `${key}: ${String(value)}`)
    .join(", ");
}

export function formatValidationIssue(issue: ValidationIssue): string {
  if (issue.code === "card.code_outside_recommended_range") {
    const min = issue.params.recommended_min;
    const max = issue.params.recommended_max;
    return `Code is outside the recommended custom range (${min} - ${max}).`;
  }

  if (issue.code === "card.code_gap_too_small") {
    const gap = issue.params.nearest_gap;
    const minGap = issue.params.min_gap;
    return `Code is very close to an existing card code (gap ${gap}, recommended at least ${minGap}).`;
  }

  const formattedParams = formatIssueParams(issue.params);
  if (!formattedParams) return issue.code;
  return `${issue.code} (${formattedParams})`;
}
