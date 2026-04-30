import type { AppError } from "../api/invoke";
import type { ValidationIssue } from "../contracts/common";
import type { JobSnapshot, JobStatus } from "../contracts/job";
import { formatAppMessage, formatAppMessageById } from "../i18n";

type MessageValues = Record<string, string | number | null | undefined>;

export interface UiMessageDescriptor {
  id: string;
  defaultMessage: string;
  values?: MessageValues;
}

interface MessageDefinition {
  id: string;
  defaultMessage: string;
}

const ERROR_MESSAGES: Record<string, MessageDefinition> = {
  "frontend.invoke_failed": {
    id: "error.frontend.invokeFailed",
    defaultMessage: "The desktop command failed. Try the action again.",
  },
  "workspace.not_open": {
    id: "error.workspace.notOpen",
    defaultMessage: "Open a workspace before using this action.",
  },
  "workspace.mismatch": {
    id: "error.workspace.mismatch",
    defaultMessage: "This action belongs to a different workspace. Reopen the workspace and try again.",
  },
  "workspace.name_required": {
    id: "error.workspace.nameRequired",
    defaultMessage: "Workspace name is required.",
  },
  "workspace.path_required": {
    id: "error.workspace.pathRequired",
    defaultMessage: "Workspace path is required.",
  },
  "workspace.root_create_failed": {
    id: "error.workspace.rootCreateFailed",
    defaultMessage: "The workspace folder could not be created. Check the path and permissions.",
  },
  "pack.not_open": {
    id: "error.pack.notOpen",
    defaultMessage: "Open this pack before using this action.",
  },
  "pack.not_found": {
    id: "error.pack.notFound",
    defaultMessage: "The pack could not be found in the current workspace.",
  },
  "pack.metadata_missing": {
    id: "error.pack.metadataMissing",
    defaultMessage: "This folder is missing pack metadata.",
  },
  "pack.duplicate_id": {
    id: "error.pack.duplicateId",
    defaultMessage: "Another pack in this workspace already uses this pack identity.",
  },
  "pack.delete_failed": {
    id: "error.pack.deleteFailed",
    defaultMessage: "The pack could not be deleted. Check file permissions and try again.",
  },
  "card.not_found": {
    id: "error.card.notFound",
    defaultMessage: "The card could not be found in the current pack.",
  },
  "config.validation_failed": {
    id: "error.config.validationFailed",
    defaultMessage: "Settings contain invalid values. Review the highlighted fields and try again.",
  },
  "confirmation.invalid_token": {
    id: "error.confirmation.invalidToken",
    defaultMessage: "This confirmation is no longer available. Review the change and try again.",
  },
  "confirmation.stale_revision": {
    id: "error.confirmation.staleRevision",
    defaultMessage: "The pack changed after this confirmation was created. Review the latest data and try again.",
  },
  "confirmation.stale_source_stamp": {
    id: "error.confirmation.staleSourceStamp",
    defaultMessage: "Files on disk changed after this confirmation was created. Reload and try again.",
  },
  "resource.external_editor_not_configured": {
    id: "error.resource.externalEditorNotConfigured",
    defaultMessage: "External text editor is not configured. Set it in Global Settings.",
  },
  "resource.external_editor_missing": {
    id: "error.resource.externalEditorMissing",
    defaultMessage: "The configured external editor could not be found.",
  },
  "resource.external_editor_launch_failed": {
    id: "error.resource.externalEditorLaunchFailed",
    defaultMessage: "The external editor could not be launched.",
  },
  "resource.script_exists": {
    id: "error.resource.scriptExists",
    defaultMessage: "This card already has a script.",
  },
  "resource.script_missing": {
    id: "error.resource.scriptMissing",
    defaultMessage: "This card does not have a script yet.",
  },
  "import.preview_token_invalid": {
    id: "error.import.previewTokenInvalid",
    defaultMessage: "This import preview is no longer available. Preview the import again.",
  },
  "import.preview_token_expired": {
    id: "error.import.previewTokenExpired",
    defaultMessage: "This import preview expired. Preview the import again.",
  },
  "import.preview_stale": {
    id: "error.import.previewStale",
    defaultMessage: "The import source changed after preview. Preview the import again.",
  },
  "import.preview_has_errors": {
    id: "error.import.previewHasErrors",
    defaultMessage: "The import still has blocking errors. Fix them before importing.",
  },
  "import.target_pack_exists": {
    id: "error.import.targetPackExists",
    defaultMessage: "A pack already exists at the import target.",
  },
  "export.pack_ids_required": {
    id: "error.export.packIdsRequired",
    defaultMessage: "Select at least one pack to export.",
  },
  "export.pack_ids_duplicate": {
    id: "error.export.packIdsDuplicate",
    defaultMessage: "The export selection contains the same pack more than once.",
  },
  "export.output_name_required": {
    id: "error.export.outputNameRequired",
    defaultMessage: "Enter an output name.",
  },
  "export.output_name_invalid": {
    id: "error.export.outputNameInvalid",
    defaultMessage: "Use a safe single folder name for the export output.",
  },
  "export.output_dir_not_empty": {
    id: "error.export.outputDirNotEmpty",
    defaultMessage: "The export output folder already exists and is not empty.",
  },
  "export.preview_token_invalid": {
    id: "error.export.previewTokenInvalid",
    defaultMessage: "This export preview is no longer available. Preview the export again.",
  },
  "export.preview_token_expired": {
    id: "error.export.previewTokenExpired",
    defaultMessage: "This export preview expired. Preview the export again.",
  },
  "export.preview_stale": {
    id: "error.export.previewStale",
    defaultMessage: "The selected packs changed after preview. Preview the export again.",
  },
  "export.preview_has_errors": {
    id: "error.export.previewHasErrors",
    defaultMessage: "The export still has blocking errors. Fix them before exporting.",
  },
  "standard_pack.ygopro_path_not_configured": {
    id: "error.standardPack.ygoproPathNotConfigured",
    defaultMessage: "Configure the YGOPro path before rebuilding the standard index.",
  },
  "standard_pack.source_language_required": {
    id: "error.standardPack.sourceLanguageRequired",
    defaultMessage: "Choose a standard pack source language before rebuilding the index.",
  },
  "standard_pack.card_not_found": {
    id: "error.standardPack.cardNotFound",
    defaultMessage: "The standard card could not be found.",
  },
  "standard_pack.cdb_missing": {
    id: "error.standardPack.cdbMissing",
    defaultMessage: "No standard CDB file was found in the configured YGOPro folder.",
  },
  "job.not_found": {
    id: "error.job.notFound",
    defaultMessage: "The task could not be found. It may have already finished or been cleared.",
  },
  "job.panic": {
    id: "error.job.panic",
    defaultMessage: "The task stopped unexpectedly.",
  },
};

const ISSUE_MESSAGES: Record<string, MessageDefinition> = {
  "card.code_required": {
    id: "issue.card.codeRequired",
    defaultMessage: "Card code is required.",
  },
  "card.code_reserved_range": {
    id: "issue.card.codeReservedRange",
    defaultMessage: "Card code is in the standard reserved range.",
  },
  "card.code_hard_max_exceeded": {
    id: "issue.card.codeHardMaxExceeded",
    defaultMessage: "Card code is above the supported maximum.",
  },
  "card.code_conflicts_with_current_pack_card": {
    id: "issue.card.codeConflictsWithCurrentPackCard",
    defaultMessage: "Another card in this pack already uses this code.",
  },
  "card.code_conflicts_with_workspace_custom_card": {
    id: "issue.card.codeConflictsWithWorkspaceCustomCard",
    defaultMessage: "Another custom pack in this workspace already uses this code.",
  },
  "card.code_conflicts_with_standard_card": {
    id: "issue.card.codeConflictsWithStandardCard",
    defaultMessage: "This code is already used by the standard card database.",
  },
  "card.code_outside_recommended_range": {
    id: "issue.card.codeOutsideRecommendedRange",
    defaultMessage: "Card code is outside the recommended custom range ({min} - {max}).",
  },
  "card.code_gap_too_small": {
    id: "issue.card.codeGapTooSmall",
    defaultMessage: "Card code is very close to an existing card code.",
  },
  "card.category_out_of_range": {
    id: "issue.card.categoryOutOfRange",
    defaultMessage: "Card category flags are out of range.",
  },
  "card.setcodes_too_many": {
    id: "issue.card.setcodesTooMany",
    defaultMessage: "A card can use at most four set codes.",
  },
  "card.texts_required": {
    id: "issue.card.textsRequired",
    defaultMessage: "Card text is required.",
  },
  "card.name_required": {
    id: "issue.card.nameRequired",
    defaultMessage: "Card name is required.",
  },
  "card.atk_invalid": {
    id: "issue.card.atkInvalid",
    defaultMessage: "ATK must be a non-negative value or unknown.",
  },
  "card.def_invalid": {
    id: "issue.card.defInvalid",
    defaultMessage: "DEF must be a non-negative value or unknown.",
  },
  "card.monster_flags_required": {
    id: "issue.card.monsterFlagsRequired",
    defaultMessage: "Monster cards need at least one monster type flag.",
  },
  "card.monster_atk_required": {
    id: "issue.card.monsterAtkRequired",
    defaultMessage: "Monster cards need ATK.",
  },
  "card.monster_race_required": {
    id: "issue.card.monsterRaceRequired",
    defaultMessage: "Monster cards need a race.",
  },
  "card.monster_attribute_required": {
    id: "issue.card.monsterAttributeRequired",
    defaultMessage: "Monster cards need an attribute.",
  },
  "card.link_markers_required": {
    id: "issue.card.linkMarkersRequired",
    defaultMessage: "Link monsters need link markers.",
  },
  "card.link_monster_def_forbidden": {
    id: "issue.card.linkMonsterDefForbidden",
    defaultMessage: "Link monsters cannot have DEF.",
  },
  "card.level_required": {
    id: "issue.card.levelRequired",
    defaultMessage: "Non-link monsters need a level or rank.",
  },
  "card.pendulum_forbidden_without_flag": {
    id: "issue.card.pendulumForbiddenWithoutFlag",
    defaultMessage: "Pendulum data requires the Pendulum monster flag.",
  },
  "card.spell_subtype_required": {
    id: "issue.card.spellSubtypeRequired",
    defaultMessage: "Spell cards need a subtype.",
  },
  "card.trap_subtype_required": {
    id: "issue.card.trapSubtypeRequired",
    defaultMessage: "Trap cards need a subtype.",
  },
  "pack.name_required": {
    id: "issue.pack.nameRequired",
    defaultMessage: "Pack name is required.",
  },
  "pack.author_required": {
    id: "issue.pack.authorRequired",
    defaultMessage: "Pack author is required.",
  },
  "pack.version_required": {
    id: "issue.pack.versionRequired",
    defaultMessage: "Pack version is required.",
  },
  "pack.display_language_order_empty": {
    id: "issue.pack.displayLanguageOrderEmpty",
    defaultMessage: "Add at least one display language for this pack.",
  },
  "pack_strings.overwrite_existing_value": {
    id: "issue.packStrings.overwriteExistingValue",
    defaultMessage: "This will overwrite an existing string value.",
  },
  "pack_strings.overwrite_existing_record": {
    id: "issue.packStrings.overwriteExistingRecord",
    defaultMessage: "This will overwrite an existing string record.",
  },
  "pack_strings.setname_base_outside_recommended_range": {
    id: "issue.packStrings.setnameBaseOutsideRecommendedRange",
    defaultMessage: "Setname base is outside the recommended custom range ({min} - {max}).",
  },
  "pack_strings.counter_key_outside_recommended_range": {
    id: "issue.packStrings.counterKeyOutsideRecommendedRange",
    defaultMessage: "Counter key is outside the recommended custom range ({min} - {max}).",
  },
  "pack_strings.victory_key_outside_recommended_range": {
    id: "issue.packStrings.victoryKeyOutsideRecommendedRange",
    defaultMessage: "Victory key is outside the recommended custom range ({min} - {max}).",
  },
  "pack_strings.system_key_conflicts_with_workspace_custom_pack": {
    id: "issue.packStrings.systemKeyConflictsWithWorkspaceCustomPack",
    defaultMessage: "Another custom pack already uses this system string key.",
  },
  "pack_strings.system_key_conflicts_with_standard_pack": {
    id: "issue.packStrings.systemKeyConflictsWithStandardPack",
    defaultMessage: "The standard pack already uses this system string key.",
  },
  "pack_strings.victory_key_conflicts_with_workspace_custom_pack": {
    id: "issue.packStrings.victoryKeyConflictsWithWorkspaceCustomPack",
    defaultMessage: "Another custom pack already uses this victory string key.",
  },
  "pack_strings.victory_key_conflicts_with_standard_pack": {
    id: "issue.packStrings.victoryKeyConflictsWithStandardPack",
    defaultMessage: "The standard pack already uses this victory string key.",
  },
  "pack_strings.counter_key_conflicts_with_workspace_custom_pack": {
    id: "issue.packStrings.counterKeyConflictsWithWorkspaceCustomPack",
    defaultMessage: "Another custom pack already uses this counter string key.",
  },
  "pack_strings.counter_key_conflicts_with_standard_pack": {
    id: "issue.packStrings.counterKeyConflictsWithStandardPack",
    defaultMessage: "The standard pack already uses this counter string key.",
  },
  "pack_strings.setname_base_conflicts_with_workspace_custom_pack": {
    id: "issue.packStrings.setnameBaseConflictsWithWorkspaceCustomPack",
    defaultMessage: "Another custom pack already uses this setname base.",
  },
  "pack_strings.setname_base_conflicts_with_standard_pack": {
    id: "issue.packStrings.setnameBaseConflictsWithStandardPack",
    defaultMessage: "The standard pack already uses this setname base.",
  },
  "import.source_language_not_in_display_order": {
    id: "issue.import.sourceLanguageNotInDisplayOrder",
    defaultMessage: "The source language is not in the display language order.",
  },
  "import.target_pack_exists": {
    id: "issue.import.targetPackExists",
    defaultMessage: "The target pack already exists.",
  },
  "import.cdb_duplicate_code": {
    id: "issue.import.cdbDuplicateCode",
    defaultMessage: "The source CDB contains duplicate card codes.",
  },
  "import.missing_main_image": {
    id: "issue.import.missingMainImage",
    defaultMessage: "Main image is missing for an imported card.",
  },
  "import.missing_script": {
    id: "issue.import.missingScript",
    defaultMessage: "Script file is missing for an imported card.",
  },
  "import.missing_field_image": {
    id: "issue.import.missingFieldImage",
    defaultMessage: "Field image is missing for an imported field spell.",
  },
  "export.output_dir_not_empty": {
    id: "issue.export.outputDirNotEmpty",
    defaultMessage: "The output folder already exists and is not empty.",
  },
  "export.pack_kind_not_supported": {
    id: "issue.export.packKindNotSupported",
    defaultMessage: "Only custom packs can be exported.",
  },
  "export.card_text_missing_target_language": {
    id: "issue.export.cardTextMissingTargetLanguage",
    defaultMessage: "A card is missing text for the export language.",
  },
  "export.pack_string_missing_target_language": {
    id: "issue.export.packStringMissingTargetLanguage",
    defaultMessage: "A string entry is missing text for the export language.",
  },
  "export.system_key_conflicts_with_standard_pack": {
    id: "issue.export.systemKeyConflictsWithStandardPack",
    defaultMessage: "A system string key conflicts with the standard pack.",
  },
  "export.setname_key_conflicts_with_standard_pack": {
    id: "issue.export.setnameKeyConflictsWithStandardPack",
    defaultMessage: "A setname key conflicts with the standard pack.",
  },
  "export.setname_base_overlaps_standard_pack": {
    id: "issue.export.setnameBaseOverlapsStandardPack",
    defaultMessage: "A setname base overlaps with the standard pack.",
  },
  "export.counter_key_conflicts_with_standard_pack": {
    id: "issue.export.counterKeyConflictsWithStandardPack",
    defaultMessage: "A counter string key conflicts with the standard pack.",
  },
  "export.victory_key_conflicts_with_standard_pack": {
    id: "issue.export.victoryKeyConflictsWithStandardPack",
    defaultMessage: "A victory string key conflicts with the standard pack.",
  },
  "export.code_conflicts_between_selected_packs": {
    id: "issue.export.codeConflictsBetweenSelectedPacks",
    defaultMessage: "Selected packs contain duplicate card codes.",
  },
  "export.code_conflicts_with_standard_pack": {
    id: "issue.export.codeConflictsWithStandardPack",
    defaultMessage: "A card code conflicts with the standard card database.",
  },
  "export.code_in_standard_reserved_range": {
    id: "issue.export.codeInStandardReservedRange",
    defaultMessage: "A card code is in the standard reserved range.",
  },
  "export.setname_key_conflicts_between_selected_packs": {
    id: "issue.export.setnameKeyConflictsBetweenSelectedPacks",
    defaultMessage: "Selected packs contain duplicate setname keys.",
  },
  "export.setname_base_overlaps_between_selected_packs": {
    id: "issue.export.setnameBaseOverlapsBetweenSelectedPacks",
    defaultMessage: "Selected packs contain overlapping setname bases.",
  },
  "export.counter_key_conflicts_between_selected_packs": {
    id: "issue.export.counterKeyConflictsBetweenSelectedPacks",
    defaultMessage: "Selected packs contain duplicate counter string keys.",
  },
  "export.victory_key_conflicts_between_selected_packs": {
    id: "issue.export.victoryKeyConflictsBetweenSelectedPacks",
    defaultMessage: "Selected packs contain duplicate victory string keys.",
  },
};

const JOB_STATUS_MESSAGES: Record<JobStatus, MessageDefinition> = {
  pending: { id: "job.status.pending", defaultMessage: "Pending" },
  running: { id: "job.status.running", defaultMessage: "Running" },
  succeeded: { id: "job.status.succeeded", defaultMessage: "Done" },
  failed: { id: "job.status.failed", defaultMessage: "Failed" },
  cancelled: { id: "job.status.cancelled", defaultMessage: "Cancelled" },
};

const JOB_STAGE_MESSAGES: Record<string, MessageDefinition> = {
  pending: { id: "job.stage.pending", defaultMessage: "Waiting to start" },
  running: { id: "job.stage.running", defaultMessage: "Starting" },
  succeeded: { id: "job.stage.succeeded", defaultMessage: "Completed" },
  failed: { id: "job.stage.failed", defaultMessage: "Stopped" },
  discover_source: { id: "job.stage.discoverSource", defaultMessage: "Locating source files" },
  build_index: { id: "job.stage.buildIndex", defaultMessage: "Reading cards and strings" },
  write_index: { id: "job.stage.writeIndex", defaultMessage: "Saving index cache" },
  refresh_cache: { id: "job.stage.refreshCache", defaultMessage: "Refreshing index cache" },
  index_ready: { id: "job.stage.indexReady", defaultMessage: "Standard index is ready" },
  validating_preview: { id: "job.stage.validatingPreview", defaultMessage: "Checking preview" },
  writing_pack: { id: "job.stage.writingPack", defaultMessage: "Writing pack files" },
  refreshing_workspace: { id: "job.stage.refreshingWorkspace", defaultMessage: "Refreshing workspace" },
  import_ready: { id: "job.stage.importReady", defaultMessage: "Import is complete" },
  writing_export: { id: "job.stage.writingExport", defaultMessage: "Writing export files" },
  export_ready: { id: "job.stage.exportReady", defaultMessage: "Export is complete" },
};

export function formatUiMessage(descriptor: UiMessageDescriptor): string {
  return formatAppMessage(
    {
      id: descriptor.id,
      defaultMessage: descriptor.defaultMessage,
    },
    sanitizeValues(descriptor.values ?? {}),
  );
}

export function formatUserError(error: unknown): string {
  return formatUiMessage(errorToMessageDescriptor(error));
}

export function formatUserIssue(issue: ValidationIssue): string {
  return formatUiMessage(issueToMessageDescriptor(issue));
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
  return formatUiMessage(messageToDescriptor(JOB_STATUS_MESSAGES[status]));
}

export function formatJobStage(stage: string): string {
  const definition = JOB_STAGE_MESSAGES[stage] ?? {
    id: "job.stage.unknown",
    defaultMessage: "Working",
  };
  return formatUiMessage(messageToDescriptor(definition));
}

export function formatJobError(job: JobSnapshot): string | null {
  return job.error ? formatUserError(job.error) : null;
}

function errorToMessageDescriptor(error: unknown): UiMessageDescriptor {
  if (isAppError(error)) {
    const exact = ERROR_MESSAGES[error.code];
    if (exact) return messageToDescriptor(exact, valuesFromDetails(error.details));

    if (error.code.startsWith("confirmation.")) {
      return {
        id: "error.confirmation.generic",
        defaultMessage: "The pending confirmation is no longer valid. Review the change and try again.",
      };
    }
    if (error.code.startsWith("standard_pack.")) {
      return {
        id: "error.standardPack.generic",
        defaultMessage: "The standard pack could not be read. Check the configured YGOPro path.",
      };
    }
    if (error.code.startsWith("resource.image_")) {
      return {
        id: "error.resource.imageGeneric",
        defaultMessage: "The image could not be processed. Try another image file.",
      };
    }
    if (error.code.startsWith("json_store.") || error.code.startsWith("fs.")) {
      return {
        id: "error.files.generic",
        defaultMessage: "Data files could not be read or written. Check file permissions and try again.",
      };
    }
    if (error.code.startsWith("ygopro_cdb.")) {
      return {
        id: "error.ygoproCdb.generic",
        defaultMessage: "The CDB file could not be read or written.",
      };
    }
    if (error.code.endsWith("_lock_poisoned")) {
      return {
        id: "error.state.temporarilyUnavailable",
        defaultMessage: "Application state is temporarily unavailable. Restart the app if this keeps happening.",
      };
    }

    return {
      id: "error.generic.app",
      defaultMessage: "The action could not be completed. Try again or review the current workspace state.",
    };
  }

  if (error instanceof Error) {
    return {
      id: "error.generic.exception",
      defaultMessage: error.message,
    };
  }

  return {
    id: "error.generic.unknown",
    defaultMessage: "An unknown error occurred.",
  };
}

function issueToMessageDescriptor(issue: ValidationIssue): UiMessageDescriptor {
  const exact = ISSUE_MESSAGES[issue.code];
  if (exact) return messageToDescriptor(exact, issueValues(issue));

  if (issue.code.endsWith("_invalid")) {
    return {
      id: "issue.language.invalid",
      defaultMessage: "A language value is not available in the configured language list.",
    };
  }

  if (issue.code.endsWith("_default_reserved")) {
    return {
      id: "issue.language.defaultReserved",
      defaultMessage: "The default language entry is reserved and cannot be used here.",
    };
  }

  return {
    id: issue.level === "error" ? "issue.generic.error" : "issue.generic.warning",
    defaultMessage:
      issue.level === "error"
        ? "Fix this issue before continuing."
        : "Review this warning before continuing.",
  };
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

function messageToDescriptor(
  definition: MessageDefinition,
  values?: MessageValues,
): UiMessageDescriptor {
  return {
    id: definition.id,
    defaultMessage: definition.defaultMessage,
    values,
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
