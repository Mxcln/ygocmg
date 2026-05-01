import type { PackId, LanguageCode } from "./common";

export type PackKind = "standard" | "custom";

export interface PackMetadata {
  id: PackId;
  kind: PackKind;
  name: string;
  pack_code: string | null;
  author: string;
  version: string;
  description: string | null;
  created_at: string;
  updated_at: string;
  display_language_order: LanguageCode[];
  default_export_language: LanguageCode | null;
}

export interface PackOverview {
  id: PackId;
  kind: PackKind;
  name: string;
  author: string;
  version: string;
  card_count: number;
  updated_at: string;
}
