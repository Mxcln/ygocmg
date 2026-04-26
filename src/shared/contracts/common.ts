export type LanguageCode = string;
export type WorkspaceId = string;
export type PackId = string;
export type CardId = string;

export interface ValidationTarget {
  scope: string;
  entity_id: string | null;
  field: string | null;
}

export interface ValidationIssue {
  code: string;
  level: "error" | "warning";
  target: ValidationTarget;
  params: Record<string, unknown>;
}
