import type { AppError } from "../api/invoke";
import type { ValidationIssue } from "../contracts/common";
import type { JobSnapshot, JobStatus } from "../contracts/job";
import { formatAppMessage, formatAppMessageById } from "../i18n";
import type { AppMessageId } from "../i18n";

type MessageValues = Record<string, string | number | null | undefined>;

const ERROR_MESSAGES: Record<string, AppMessageId> = {
  "frontend.invoke_failed": "error.frontend.invokeFailed",
  "workspace.not_open": "error.workspace.notOpen",
  "workspace.mismatch": "error.workspace.mismatch",
  "workspace.name_required": "error.workspace.nameRequired",
  "workspace.path_required": "error.workspace.pathRequired",
  "workspace.root_create_failed": "error.workspace.rootCreateFailed",
  "pack.not_open": "error.pack.notOpen",
  "pack.not_found": "error.pack.notFound",
  "pack.metadata_missing": "error.pack.metadataMissing",
  "pack.duplicate_id": "error.pack.duplicateId",
  "pack.delete_failed": "error.pack.deleteFailed",
  "card.not_found": "error.card.notFound",
  "config.validation_failed": "error.config.validationFailed",
  "confirmation.invalid_token": "error.confirmation.invalidToken",
  "confirmation.stale_revision": "error.confirmation.staleRevision",
  "confirmation.stale_source_stamp": "error.confirmation.staleSourceStamp",
  "resource.external_editor_not_configured": "error.resource.externalEditorNotConfigured",
  "resource.external_editor_missing": "error.resource.externalEditorMissing",
  "resource.external_editor_launch_failed": "error.resource.externalEditorLaunchFailed",
  "resource.script_exists": "error.resource.scriptExists",
  "resource.script_missing": "error.resource.scriptMissing",
  "import.preview_token_invalid": "error.import.previewTokenInvalid",
  "import.preview_token_expired": "error.import.previewTokenExpired",
  "import.preview_stale": "error.import.previewStale",
  "import.preview_has_errors": "error.import.previewHasErrors",
  "import.target_pack_exists": "error.import.targetPackExists",
  "export.pack_ids_required": "error.export.packIdsRequired",
  "export.pack_ids_duplicate": "error.export.packIdsDuplicate",
  "export.output_name_required": "error.export.outputNameRequired",
  "export.output_name_invalid": "error.export.outputNameInvalid",
  "export.output_dir_not_empty": "error.export.outputDirNotEmpty",
  "export.preview_token_invalid": "error.export.previewTokenInvalid",
  "export.preview_token_expired": "error.export.previewTokenExpired",
  "export.preview_stale": "error.export.previewStale",
  "export.preview_has_errors": "error.export.previewHasErrors",
  "standard_pack.ygopro_path_not_configured": "error.standardPack.ygoproPathNotConfigured",
  "standard_pack.source_language_required": "error.standardPack.sourceLanguageRequired",
  "standard_pack.card_not_found": "error.standardPack.cardNotFound",
  "standard_pack.cdb_missing": "error.standardPack.cdbMissing",
  "job.not_found": "error.job.notFound",
  "job.panic": "error.job.panic",
};

const ISSUE_MESSAGES: Record<string, AppMessageId> = {
  "card.code_required": "issue.card.codeRequired",
  "card.code_reserved_range": "issue.card.codeReservedRange",
  "card.code_hard_max_exceeded": "issue.card.codeHardMaxExceeded",
  "card.code_conflicts_with_current_pack_card": "issue.card.codeConflictsWithCurrentPackCard",
  "card.code_conflicts_with_workspace_custom_card": "issue.card.codeConflictsWithWorkspaceCustomCard",
  "card.code_conflicts_with_standard_card": "issue.card.codeConflictsWithStandardCard",
  "card.code_outside_recommended_range": "issue.card.codeOutsideRecommendedRange",
  "card.code_gap_too_small": "issue.card.codeGapTooSmall",
  "card.category_out_of_range": "issue.card.categoryOutOfRange",
  "card.setcodes_too_many": "issue.card.setcodesTooMany",
  "card.texts_required": "issue.card.textsRequired",
  "card.name_required": "issue.card.nameRequired",
  "card.atk_invalid": "issue.card.atkInvalid",
  "card.def_invalid": "issue.card.defInvalid",
  "card.monster_flags_required": "issue.card.monsterFlagsRequired",
  "card.monster_atk_required": "issue.card.monsterAtkRequired",
  "card.monster_race_required": "issue.card.monsterRaceRequired",
  "card.monster_attribute_required": "issue.card.monsterAttributeRequired",
  "card.link_markers_required": "issue.card.linkMarkersRequired",
  "card.link_monster_def_forbidden": "issue.card.linkMonsterDefForbidden",
  "card.level_required": "issue.card.levelRequired",
  "card.pendulum_forbidden_without_flag": "issue.card.pendulumForbiddenWithoutFlag",
  "card.spell_subtype_required": "issue.card.spellSubtypeRequired",
  "card.trap_subtype_required": "issue.card.trapSubtypeRequired",
  "pack.name_required": "issue.pack.nameRequired",
  "pack.author_required": "issue.pack.authorRequired",
  "pack.version_required": "issue.pack.versionRequired",
  "pack.display_language_order_empty": "issue.pack.displayLanguageOrderEmpty",
  "pack_strings.overwrite_existing_value": "issue.packStrings.overwriteExistingValue",
  "pack_strings.overwrite_existing_record": "issue.packStrings.overwriteExistingRecord",
  "pack_strings.setname_base_outside_recommended_range": "issue.packStrings.setnameBaseOutsideRecommendedRange",
  "pack_strings.counter_key_outside_recommended_range": "issue.packStrings.counterKeyOutsideRecommendedRange",
  "pack_strings.victory_key_outside_recommended_range": "issue.packStrings.victoryKeyOutsideRecommendedRange",
  "pack_strings.system_key_conflicts_with_workspace_custom_pack": "issue.packStrings.systemKeyConflictsWithWorkspaceCustomPack",
  "pack_strings.system_key_conflicts_with_standard_pack": "issue.packStrings.systemKeyConflictsWithStandardPack",
  "pack_strings.victory_key_conflicts_with_workspace_custom_pack": "issue.packStrings.victoryKeyConflictsWithWorkspaceCustomPack",
  "pack_strings.victory_key_conflicts_with_standard_pack": "issue.packStrings.victoryKeyConflictsWithStandardPack",
  "pack_strings.counter_key_conflicts_with_workspace_custom_pack": "issue.packStrings.counterKeyConflictsWithWorkspaceCustomPack",
  "pack_strings.counter_key_conflicts_with_standard_pack": "issue.packStrings.counterKeyConflictsWithStandardPack",
  "pack_strings.setname_base_conflicts_with_workspace_custom_pack": "issue.packStrings.setnameBaseConflictsWithWorkspaceCustomPack",
  "pack_strings.setname_base_conflicts_with_standard_pack": "issue.packStrings.setnameBaseConflictsWithStandardPack",
  "import.source_language_not_in_display_order": "issue.import.sourceLanguageNotInDisplayOrder",
  "import.target_pack_exists": "issue.import.targetPackExists",
  "import.cdb_duplicate_code": "issue.import.cdbDuplicateCode",
  "import.missing_main_image": "issue.import.missingMainImage",
  "import.missing_script": "issue.import.missingScript",
  "import.missing_field_image": "issue.import.missingFieldImage",
  "export.output_dir_not_empty": "issue.export.outputDirNotEmpty",
  "export.pack_kind_not_supported": "issue.export.packKindNotSupported",
  "export.card_text_missing_target_language": "issue.export.cardTextMissingTargetLanguage",
  "export.pack_string_missing_target_language": "issue.export.packStringMissingTargetLanguage",
  "export.system_key_conflicts_with_standard_pack": "issue.export.systemKeyConflictsWithStandardPack",
  "export.setname_key_conflicts_with_standard_pack": "issue.export.setnameKeyConflictsWithStandardPack",
  "export.setname_base_overlaps_standard_pack": "issue.export.setnameBaseOverlapsStandardPack",
  "export.counter_key_conflicts_with_standard_pack": "issue.export.counterKeyConflictsWithStandardPack",
  "export.victory_key_conflicts_with_standard_pack": "issue.export.victoryKeyConflictsWithStandardPack",
  "export.code_conflicts_between_selected_packs": "issue.export.codeConflictsBetweenSelectedPacks",
  "export.code_conflicts_with_standard_pack": "issue.export.codeConflictsWithStandardPack",
  "export.code_in_standard_reserved_range": "issue.export.codeInStandardReservedRange",
  "export.setname_key_conflicts_between_selected_packs": "issue.export.setnameKeyConflictsBetweenSelectedPacks",
  "export.setname_base_overlaps_between_selected_packs": "issue.export.setnameBaseOverlapsBetweenSelectedPacks",
  "export.counter_key_conflicts_between_selected_packs": "issue.export.counterKeyConflictsBetweenSelectedPacks",
  "export.victory_key_conflicts_between_selected_packs": "issue.export.victoryKeyConflictsBetweenSelectedPacks",
};

const JOB_STATUS_MESSAGES: Record<JobStatus, AppMessageId> = {
  pending: "job.status.pending",
  running: "job.status.running",
  succeeded: "job.status.succeeded",
  failed: "job.status.failed",
  cancelled: "job.status.cancelled",
};

const JOB_STAGE_MESSAGES: Record<string, AppMessageId> = {
  pending: "job.stage.pending",
  running: "job.stage.running",
  succeeded: "job.stage.succeeded",
  failed: "job.stage.failed",
  discover_source: "job.stage.discoverSource",
  build_index: "job.stage.buildIndex",
  write_index: "job.stage.writeIndex",
  refresh_cache: "job.stage.refreshCache",
  index_ready: "job.stage.indexReady",
  validating_preview: "job.stage.validatingPreview",
  writing_pack: "job.stage.writingPack",
  refreshing_workspace: "job.stage.refreshingWorkspace",
  import_ready: "job.stage.importReady",
  writing_export: "job.stage.writingExport",
  export_ready: "job.stage.exportReady",
};

export function formatUserError(error: unknown): string {
  const id = errorToMessageId(error);
  const values = isAppError(error) ? sanitizeValues(valuesFromDetails(error.details)) : {};
  return formatAppMessageById(id, values);
}

export function formatUserIssue(issue: ValidationIssue): string {
  const id = issueToMessageId(issue);
  const values = sanitizeValues(issueValues(issue));
  return formatAppMessageById(id, values);
}

export function formatIssueDetail(issue: ValidationIssue): string | null {
  const parts = [
    formatCardCodeDetail(issue),
    formatStringKeyDetail(issue),
    formatLanguageDetail(issue),
    formatCountDetail(issue),
    formatPathDetail(issue),
    formatPackListDetail(issue),
  ].filter((part): part is string => Boolean(part));

  return parts.length > 0 ? parts.join(" | ") : null;
}

export function formatIssueLevel(level: ValidationIssue["level"]): string {
  return level === "error" ? formatAppMessageById("common.error") : formatAppMessageById("common.warning");
}

export function formatJobStatus(status: JobStatus): string {
  return formatAppMessageById(JOB_STATUS_MESSAGES[status]);
}

export function formatJobStage(stage: string): string {
  const id = JOB_STAGE_MESSAGES[stage] ?? ("job.stage.unknown" as AppMessageId);
  return formatAppMessageById(id);
}

export function formatJobError(job: JobSnapshot): string | null {
  return job.error ? formatUserError(job.error) : null;
}

function errorToMessageId(error: unknown): AppMessageId {
  if (isAppError(error)) {
    const exact = ERROR_MESSAGES[error.code];
    if (exact) return exact;

    if (error.code.startsWith("confirmation.")) return "error.confirmation.generic" as AppMessageId;
    if (error.code.startsWith("standard_pack.")) return "error.standardPack.generic" as AppMessageId;
    if (error.code.startsWith("resource.image_")) return "error.resource.imageGeneric" as AppMessageId;
    if (error.code.startsWith("json_store.") || error.code.startsWith("fs.")) return "error.files.generic" as AppMessageId;
    if (error.code.startsWith("ygopro_cdb.")) return "error.ygoproCdb.generic" as AppMessageId;
    if (error.code.endsWith("_lock_poisoned")) return "error.state.temporarilyUnavailable" as AppMessageId;

    return "error.generic.app" as AppMessageId;
  }

  if (error instanceof Error) {
    return "error.generic.app" as AppMessageId;
  }

  return "error.generic.unknown" as AppMessageId;
}

function issueToMessageId(issue: ValidationIssue): AppMessageId {
  const exact = ISSUE_MESSAGES[issue.code];
  if (exact) return exact;

  if (issue.code.endsWith("_invalid")) return "issue.language.invalid" as AppMessageId;
  if (issue.code.endsWith("_default_reserved")) return "issue.language.defaultReserved" as AppMessageId;

  return (issue.level === "error" ? "issue.generic.error" : "issue.generic.warning") as AppMessageId;
}

function issueValues(issue: ValidationIssue): MessageValues {
  return {
    min:
      issue.params.recommended_min !== undefined
        ? formatGeneralValue(issue.params.recommended_min)
        : formatGeneralValue(issue.params.recommended_base_min ?? issue.params.recommended_low12_min),
    max:
      issue.params.recommended_max !== undefined
        ? formatGeneralValue(issue.params.recommended_max)
        : formatGeneralValue(issue.params.recommended_base_max ?? issue.params.recommended_low12_max),
  };
}

function isAppError(value: unknown): value is AppError {
  return typeof value === "object" && value !== null && "code" in value && "message" in value;
}

function valuesFromDetails(details: AppError["details"]): MessageValues {
  if (!details || typeof details !== "object" || Array.isArray(details)) return {};
  return Object.fromEntries(
    Object.entries(details).map(([key, value]) => [key, formatGeneralValue(value)]),
  );
}

function formatCardCodeDetail(issue: ValidationIssue): string | null {
  const value = issue.params.code;
  if (value === undefined) return null;
  return formatAppMessageById("common.cardCode", { code: formatGeneralValue(value) });
}

function formatStringKeyDetail(issue: ValidationIssue): string | null {
  const kind = issue.params.kind;
  const key = issue.params.key;
  const base = issue.params.base;

  if (kind !== undefined && key !== undefined) {
    return `${formatStringKind(kind)} ${formatHexValue(key)}`;
  }
  if (key !== undefined) return formatAppMessageById("common.key", { key: formatHexValue(key) });
  if (base !== undefined) return formatAppMessageById("common.base", { base: formatHexValue(base) });
  return null;
}

function formatLanguageDetail(issue: ValidationIssue): string | null {
  const language = issue.params.language ?? issue.params.source_language;
  if (language === undefined) return null;
  return formatAppMessageById("common.language", { language: formatGeneralValue(language) });
}

function formatCountDetail(issue: ValidationIssue): string | null {
  if (issue.params.count === undefined) return null;
  return formatAppMessageById("common.count", { count: formatGeneralValue(issue.params.count) });
}

function formatPathDetail(issue: ValidationIssue): string | null {
  if (issue.params.path === undefined) return null;
  return formatAppMessageById("common.path", { path: formatGeneralValue(issue.params.path) });
}

function formatPackListDetail(issue: ValidationIssue): string | null {
  const packIds = issue.params.pack_ids;
  if (!Array.isArray(packIds) || packIds.length === 0) return null;
  return formatAppMessageById("common.packs", {
    packs: packIds.map((value) => formatGeneralValue(value)).join(", "),
  });
}

function formatStringKind(value: unknown): string {
  const normalized = String(value).toLowerCase();
  if (normalized.includes("setname")) return formatAppMessageById("common.stringKind.setname");
  if (normalized.includes("counter")) return formatAppMessageById("common.stringKind.counter");
  if (normalized.includes("victory")) return formatAppMessageById("common.stringKind.victory");
  if (normalized.includes("system")) return formatAppMessageById("common.stringKind.system");
  return formatAppMessageById("common.stringKind.string");
}

function formatGeneralValue(value: unknown): string {
  if (value === null || value === undefined) return "";
  if (Array.isArray(value)) return value.map(formatGeneralValue).join(", ");
  if (typeof value === "object") {
    return Object.values(value as Record<string, unknown>).map(formatGeneralValue).join(", ");
  }
  return String(value);
}

function formatHexValue(value: unknown): string {
  if (typeof value === "number" && Number.isInteger(value)) {
    return `0x${value.toString(16).toUpperCase()}`;
  }
  if (typeof value === "string" && /^\d+$/.test(value)) {
    return `0x${Number.parseInt(value, 10).toString(16).toUpperCase()}`;
  }
  return formatGeneralValue(value);
}

function sanitizeValues(values: MessageValues): Record<string, string | number> {
  return Object.fromEntries(
    Object.entries(values).map(([key, value]) => [
      key,
      value === null || value === undefined ? "" : value,
    ]),
  );
}
