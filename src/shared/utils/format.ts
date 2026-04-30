import type { ValidationIssue } from "../contracts/common";
import { formatAppMessageById, getActiveAppLocale } from "../i18n";
import { formatUserError, formatUserIssue } from "./messages";

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
  if (!value) return formatAppMessageById("common.noRecordedTime");

  const timestamp = new Date(value);
  if (Number.isNaN(timestamp.getTime())) return value;

  return timestamp.toLocaleString(getActiveAppLocale(), { hour12: false });
}

export function formatError(error: unknown): string {
  return formatUserError(error);
}

export function formatValidationIssue(issue: ValidationIssue): string {
  return formatUserIssue(issue);
}
