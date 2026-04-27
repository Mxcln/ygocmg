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

export function parseHexInput(value: string): number {
  const normalized = value.trim().replace(/^0x/i, "");
  if (!normalized) return Number.NaN;
  const parsed = Number.parseInt(normalized, 16);
  return Number.isFinite(parsed) ? parsed : Number.NaN;
}

export function formatStringKeyHex(value: number): string {
  return `0x${value.toString(16).toUpperCase()}`;
}

function isIntegerNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value);
}

function formatHexValue(value: unknown): string {
  if (isIntegerNumber(value)) return formatStringKeyHex(value);
  if (typeof value === "string" && /^\d+$/.test(value)) {
    return formatStringKeyHex(Number.parseInt(value, 10));
  }
  return String(value);
}

function formatHexList(value: unknown): string {
  if (Array.isArray(value)) {
    return value.map((item) => formatHexValue(item)).join(", ");
  }
  return formatHexValue(value);
}

function formatOwnersMap(value: unknown): string {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return String(value);
  }

  return Object.entries(value as Record<string, unknown>)
    .map(([key, owners]) => {
      const ownerList = Array.isArray(owners) ? owners.join(", ") : String(owners);
      return `${formatHexValue(key)} => [${ownerList}]`;
    })
    .join("; ");
}

function shouldFormatHex(issueCode: string, paramKey: string): boolean {
  if (issueCode.startsWith("pack_strings.") || issueCode.startsWith("export.")) {
    return [
      "key",
      "base",
      "recommended_min",
      "recommended_max",
      "recommended_base_min",
      "recommended_base_max",
      "recommended_low12_min",
      "recommended_low12_max",
    ].includes(paramKey);
  }

  if (issueCode.startsWith("card.code")) {
    return [
      "code",
      "conflicting_code",
      "recommended_min",
      "recommended_max",
      "nearest_code",
    ].includes(paramKey);
  }

  return false;
}

function formatIssueParamValue(issueCode: string, key: string, value: unknown): string {
  if (issueCode.startsWith("export.") && key.endsWith("_owners")) {
    return formatOwnersMap(value);
  }

  if (shouldFormatHex(issueCode, key)) {
    return formatHexList(value);
  }

  return String(value);
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

function formatIssueParams(issueCode: string, params: Record<string, unknown>): string {
  const entries = Object.entries(params);
  if (entries.length === 0) return "";

  return entries
    .map(([key, value]) => `${key}: ${formatIssueParamValue(issueCode, key, value)}`)
    .join(", ");
}

export function formatValidationIssue(issue: ValidationIssue): string {
  if (issue.code === "card.code_outside_recommended_range") {
    const min = formatHexValue(issue.params.recommended_min);
    const max = formatHexValue(issue.params.recommended_max);
    return `Code is outside the recommended custom range (${min} - ${max}).`;
  }

  if (issue.code === "card.code_gap_too_small") {
    const gap = issue.params.nearest_gap;
    const minGap = issue.params.min_gap;
    return `Code is very close to an existing card code (gap ${gap}, recommended at least ${minGap}).`;
  }

  if (issue.code === "pack_strings.setname_base_outside_recommended_range") {
    const base = formatHexValue(issue.params.base);
    const min = formatHexValue(issue.params.recommended_base_min);
    const max = formatHexValue(issue.params.recommended_base_max);
    return `Setname base ${base} is outside the recommended custom range (${min} - ${max}).`;
  }

  if (issue.code === "pack_strings.counter_key_outside_recommended_range") {
    const key = formatHexValue(issue.params.key);
    const min = formatHexValue(issue.params.recommended_low12_min);
    const max = formatHexValue(issue.params.recommended_low12_max);
    return `Counter key ${key} is outside the recommended custom low12 range (${min} - ${max}).`;
  }

  if (issue.code === "pack_strings.victory_key_outside_recommended_range") {
    const key = formatHexValue(issue.params.key);
    const min = formatHexValue(issue.params.recommended_min);
    const max = formatHexValue(issue.params.recommended_max);
    return `Victory key ${key} is outside the recommended custom range (${min} - ${max}).`;
  }

  const formattedParams = formatIssueParams(issue.code, issue.params);
  if (!formattedParams) return issue.code;
  return `${issue.code} (${formattedParams})`;
}
