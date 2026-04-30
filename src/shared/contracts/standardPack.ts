import type {
  CardAssetState,
  CardEntity,
  CardListRow,
  SortDirection,
} from "./card";
import type { JobAccepted } from "./job";
import type { PackStringEntry, PackStringKind } from "./strings";

export type StandardCardSortField = "code" | "name" | "type";
export type StandardStringSortField = "kind" | "key" | "value";

export type StandardPackIndexState =
  | "not_configured"
  | "missing_language"
  | "missing_source"
  | "missing_index"
  | "stale"
  | "ready"
  | "error";

export interface StandardPackStatus {
  configured: boolean;
  ygopro_path: string | null;
  cdb_path: string | null;
  index_exists: boolean;
  schema_mismatch: boolean;
  stale: boolean;
  source_language: string | null;
  indexed_at: string | null;
  card_count: number;
  state: StandardPackIndexState;
  message: string | null;
}

export interface SearchStandardCardsInput {
  keyword: string | null;
  sortBy: StandardCardSortField;
  sortDirection: SortDirection;
  page: number;
  pageSize: number;
}

export interface StandardCardPage {
  items: CardListRow[];
  page: number;
  page_size: number;
  total: number;
  ygopro_path: string | null;
  revision: number;
}

export interface GetStandardCardInput {
  code: number;
}

export interface StandardCardDetail {
  card: CardEntity;
  asset_state: CardAssetState;
  available_languages: string[];
  ygopro_path: string;
}

export interface SearchStandardStringsInput {
  kindFilter: PackStringKind | null;
  keyFilter: number | null;
  keyword: string | null;
  sortBy: StandardStringSortField;
  sortDirection: SortDirection;
  page: number;
  pageSize: number;
}

export interface StandardStringsPage {
  language: string;
  items: PackStringEntry[];
  page: number;
  page_size: number;
  total: number;
  revision: number;
}

export interface ListStandardSetnamesInput {
  language: string | null;
}

export interface StandardSetnameEntry {
  key: number;
  value: string;
}

export type StandardPackRebuildJob = JobAccepted;
