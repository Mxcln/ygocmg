import type { LanguageCode } from "./common";

export type PackStringKind = "system" | "victory" | "counter" | "setname";

export interface PackStringEntry {
  kind: PackStringKind;
  key: number;
  value: string;
}

export interface PackStringsPage {
  language: LanguageCode;
  items: PackStringEntry[];
  page: number;
  page_size: number;
  total: number;
}

export interface ListPackStringsInput {
  workspaceId: string;
  packId: string;
  language: string;
  kindFilter: PackStringKind | null;
  keyFilter: number | null;
  keyword: string | null;
  page: number;
  pageSize: number;
}

export interface UpsertPackStringInput {
  workspaceId: string;
  packId: string;
  language: string;
  entry: PackStringEntry;
}

export interface DeletePackStringsInput {
  workspaceId: string;
  packId: string;
  entries: PackStringKey[];
}

export interface PackStringKey {
  kind: PackStringKind;
  key: number;
}

export interface DeletePackStringsResult {
  deleted_count: number;
}

export interface ConfirmPackStringsWriteInput {
  confirmationToken: string;
}
