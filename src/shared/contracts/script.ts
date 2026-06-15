export type LuaValidationLevel = "static" | "ocgcore_init" | "smoke" | "scenario";

export type LuaValidationStatus = "pass" | "warning" | "fail" | "inconclusive";

export type LuaValidationConfidence = "low" | "medium" | "high";

export type LuaValidationStage = LuaValidationLevel;

export interface ValidateLuaScriptInput {
  workspaceId: string;
  packId: string;
  cardId: string;
  scriptText?: string;
  levels?: LuaValidationLevel[];
}

export interface LuaValidationIssue {
  severity: "error" | "warning" | "info";
  stage: LuaValidationStage;
  code: string;
  message: string;
  line?: number;
  column?: number;
  suggestion?: string;
}

export interface LuaValidationStageResult {
  stage: LuaValidationStage;
  status: LuaValidationStatus;
  durationMs: number;
  issues: LuaValidationIssue[];
  log?: string[];
}

export interface LuaValidationReport {
  status: LuaValidationStatus;
  confidence: LuaValidationConfidence;
  summary: string;
  issues: LuaValidationIssue[];
  stages: LuaValidationStageResult[];
  limitations: string[];
}
