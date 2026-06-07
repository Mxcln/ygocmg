# Card Text Language Design

Date: 2026-04-29

This document records the implemented card text language refactor. It replaces the earlier free-form authoring model with a managed language catalog while preserving the portable open string storage model.

## Product Decision

`LanguageCode` remains an open string in pack files and API contracts:

```rust
pub type LanguageCode = String;
```

```ts
export type LanguageCode = string;
```

The application now treats those strings as managed text language ids:

1. Settings owns the list of supported text languages.
2. Normal authoring flows select languages from that catalog.
3. `default` is reserved for legacy/parser compatibility and is not created by new authoring flows.
4. Existing packs with unknown non-`default` language ids remain readable.
5. Pack data stays portable; the global catalog is a local labeling and authoring-control layer, not the source of truth for pack contents.

## Implemented Backend Model

The backend now has `domain/language`:

```rust
pub const LEGACY_DEFAULT_LANGUAGE: &str = "default";

pub enum TextLanguageKind {
    Builtin,
    Custom,
}

pub struct TextLanguageProfile {
    pub id: LanguageCode,
    pub label: String,
    pub kind: TextLanguageKind,
    pub hidden: bool,
    pub last_used_at: Option<AppTimestamp>,
}
```

Built-in languages:

```text
zh-CN  Simplified Chinese
en-US  English
ja-JP  Japanese
ko-KR  Korean
es-ES  Spanish
```

`GlobalConfig` now stores:

```rust
pub text_language_catalog: Vec<TextLanguageProfile>;
pub standard_pack_source_language: Option<LanguageCode>;
```

The frontend contract mirrors this with:

```ts
export type TextLanguageKind = "builtin" | "custom";

export interface TextLanguageProfile {
  id: LanguageCode;
  label: string;
  kind: TextLanguageKind;
  hidden: boolean;
  last_used_at: string | null;
}
```

## Validation Rules

Language ids are normalized by trimming whitespace. Known built-ins are canonicalized by id, for example `zh-cn` can normalize to `zh-CN`.

New user-authored language ids must:

1. Not be empty.
2. Not contain leading/trailing whitespace after normalization.
3. Not contain control characters.
4. Not contain filesystem-hostile characters such as `/`, `\`, `:`, `*`, `?`, `"`, `<`, `>`, or `|`.
5. Be at most 64 characters.
6. Be BCP-ish ids such as `fr-FR`, or custom ids beginning with `x-`.
7. Not equal `default`.
8. Be present in the visible global catalog for new normal authoring writes.

Compatibility rule:

Unknown non-`default` ids already present in an opened pack/card/string record can be preserved by unrelated saves. This keeps legacy packs usable without automatically growing the user's catalog.

## Config Behavior

`ConfigService` now:

1. Injects missing built-in language profiles when loading old config files.
2. Normalizes catalog ids and labels on save.
3. Rejects duplicate ids.
4. Rejects invalid ids.
5. Rejects `default`.
6. Rejects empty labels.
7. Requires `standard_pack_source_language`, when set, to be valid, non-`default`, and visible in the catalog.

No dedicated Tauri command was added for language management. Settings continues to use `initialize`, `load_config`, and `save_config`.

## Backend Write Enforcement

The following new-write paths now validate catalog membership and reject `default`:

1. Pack creation.
2. Pack metadata update.
3. Import preview/import execute language fields.
4. Export preview/export execute language fields.
5. Card create.
6. Card update.
7. Pack string translation upsert.
8. Pack string record upsert.

Opening and browsing packs do not require every pack language to be in the local catalog.

## Standard Pack Source Language

Standard pack indexes now store a real source language:

```rust
pub source_language: LanguageCode;
```

`STANDARD_INDEX_SCHEMA_VERSION` was bumped to `3`.

The rebuild flow now requires `GlobalConfig.standard_pack_source_language`. If it is missing, rebuild fails with:

```text
standard_pack.source_language_required
```

Status now includes:

```ts
source_language: string | null
```

and can return:

```text
missing_language
```

During rebuild, parser-produced `default` card text and `strings.conf` values are remapped to the configured source language before persistence. Standard card rows, details, and standard strings search all read from that real language.

The standard pack index is considered stale when:

1. Source files changed.
2. Index schema version is old.
3. Index `source_language` differs from current config.

## Frontend Language UI

Shared frontend language utilities live in:

```text
src/shared/utils/language.ts
```

Reusable controls live in:

```text
src/features/language/TextLanguagePicker.tsx
src/features/language/LanguageOrderEditor.tsx
```

Normal business flows no longer expose free-form language id inputs. Missing custom variants must be added in Settings first.

Settings now has:

1. A Text Languages section.
2. Built-in and custom language rows.
3. Custom `x-*` language creation.
4. Custom label editing.
5. Hide/show for custom languages.
6. A Standard Pack source language picker.

## Updated Product Flows

### Create Pack

Create pack now uses `LanguageOrderEditor` for display languages and `TextLanguagePicker` for default export language.

Default authoring language prefers `en-US`, then the first visible catalog language.

### Pack Metadata

Active pack metadata editing now uses the same catalog-backed controls. Saving metadata automatically refreshes card and strings queries so list display follows a changed language order immediately.

### Import

Import source language, display languages, and default export language all use catalog-backed controls.

Default source language preference:

1. `standard_pack_source_language`
2. app language if present in the catalog
3. `zh-CN`
4. first visible catalog language

### Export

Export language uses a catalog picker.

Single-pack export preselects the pack's catalog-valid default export language. Multi-pack export preselects a default only when all selected packs share the same catalog-valid default.

Export remains strict: it requires exact target-language text and does not use display fallback.

### Card Text Editor

The card text editor now:

1. Shows expected pack languages from `display_language_order`.
2. Shows actual card languages from `draft.texts`.
3. Marks expected languages that are missing on the card.
4. Creates an empty 16-string draft when a missing expected language is clicked.
5. Adds extra languages through a catalog dropdown only.
6. Deletes the active language text with confirmation when non-empty.
7. Uses a compact language tag row with a `+` add button and a subtle `Delete` action.
8. Does not use stale `available_languages` to keep deleted languages visible.
9. Updates card detail cache after save so newly added languages are visible immediately after reopening.

If a language remains in pack display order, deleting it from one card may leave a missing expected-language tag. That is intentional.

### Card List

Card list display still uses pack `display_language_order` fallback.

The frontend query key now includes language order, and metadata save invalidates the affected card list. Changing the order from `en-US, zh-CN` to `zh-CN, en-US` refreshes the list automatically.

### Pack Strings

Pack strings keep their multilingual aggregate storage:

```rust
pub struct PackStringRecord {
    pub kind: PackStringKind,
    pub key: u32,
    pub values: BTreeMap<LanguageCode, String>,
}
```

The UI now has a single language model:

1. The top-left language picker selects the active language.
2. The list shows every shared string key.
3. The active language's value is shown in the value cell.
4. If the active language is missing, the value is an empty string and can be edited directly.
5. Clearing a value removes that language translation.
6. The row `Del` button deletes the entire string key across languages.

The old per-row expanded translation matrix was removed because it duplicated the top-level language selector and made the list too heavy.

## Compatibility and Migration

Existing config files load without migration because the catalog and standard pack source language have serde defaults.

Existing custom packs with normal ids such as `zh-CN`, `en-US`, and `ja-JP` require no migration.

Existing custom packs containing `default` should still open, but `default` is not available for new authoring. A future migration should rename `default` to a user-selected real language across:

1. Card text maps.
2. Pack string values.
3. Display language order.
4. Default export language.

Existing standard pack indexes without `source_language`, or with legacy `default`, are treated as stale through the schema/source-language checks and should be rebuilt after configuring Standard Pack source language.

## Test Coverage

Backend tests now cover:

1. Config built-in catalog injection.
2. Config validation for duplicate ids, invalid ids, `default`, and empty labels.
3. Pack create/update language validation.
4. Card create/update catalog enforcement and legacy unknown preservation.
5. Import language validation and remapping.
6. Export language validation and missing target-language checks.
7. Standard pack source-language rebuild/status behavior.
8. Old standard pack schema staleness.
9. Pack strings catalog enforcement.
10. Pack strings shared-key listing across languages.

Frontend verification currently relies on TypeScript/build checks:

```text
npm.cmd run typecheck
npm.cmd run build
```

Rust verification:

```text
cargo test --offline
```

## Remaining Work

The implemented slice intentionally leaves a few follow-ups:

1. Guided migration for custom packs containing `default`.
2. Optional "add discovered language to Settings" action for unknown non-`default` ids.
3. Export validation for target language entries with empty card names.
4. A final decision on whether display fallback should skip empty names.
5. Frontend component tests for language picker/card text/strings interactions if a frontend test runner is introduced.

## Summary

The final model is:

```text
Storage remains open:
CardEntity.texts: Record<LanguageCode, CardTexts>
PackMetadata.display_language_order: LanguageCode[]
PackMetadata.default_export_language: LanguageCode | null
PackStringRecord.values: Record<LanguageCode, string>

Authoring becomes catalog-managed:
GlobalConfig.text_language_catalog: TextLanguageProfile[]
GlobalConfig.standard_pack_source_language: LanguageCode | null
StandardPackIndexFile.source_language: LanguageCode

Legacy is explicit:
default is readable compatibility data, not a new authoring language.
```
