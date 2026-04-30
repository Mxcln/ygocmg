# UI i18n Refactor Summary

Date: 2026-04-30

This document records the first UI internationalization refactor. It is intentionally scoped to program UI text, not card text, pack text, or YGOPro data text.

## 1. Scope

Implemented in this slice:

- Add a dedicated application UI i18n layer based on `react-intl`.
- Support two UI locales:
  - `en-US`
  - `zh-CN`
- Keep `en-US` as the default UI locale.
- Use placeholder Simplified Chinese messages in the form:

```text
[待译] Save
```

- Separate `GlobalConfig.app_language` from card text languages.
- Migrate the main visible UI copy to i18n calls.
- Localize app-level notices, confirmation dialogs, validation/warning text, job status/stage text, timestamps, and card display labels.
- Keep card list `subtype_display` backend DTO unchanged.
- Translate known card subtype display parts on the frontend by splitting the existing joined string.

Explicitly not included:

- Real Simplified Chinese translation content.
- Card text migration.
- Pack strings migration.
- Standard pack index schema change.
- Backend `CardListRow` subtype restructuring.
- Migration of user-authored names, paths, card text values, pack string values, or YGOPro-specific source text.

## 2. Product Boundary

The key product decision is that UI language and card text language are different domains.

UI language:

- Stored in `GlobalConfig.app_language`.
- Controls program chrome, buttons, dialogs, status messages, labels, placeholders, and formatted timestamps.
- Currently limited to `en-US` and `zh-CN`.

Card text language:

- Stored in pack/card/string data as open `LanguageCode` strings.
- Controlled by `GlobalConfig.text_language_catalog`, pack display language order, default export language, and standard pack source language.
- Remains portable pack data and is not rewritten by this UI i18n work.

The Settings UI now reflects this split:

- `App language` uses a dedicated UI locale select.
- `Text Languages` continues to manage authoring/display/export languages for card and pack data.
- Built-in text language labels may be displayed through UI i18n, but those localized labels are not written back into config.

## 3. Frontend Architecture

New i18n files:

```text
src/shared/i18n/locales.ts
src/shared/i18n/messages.ts
src/shared/i18n/AppI18nProvider.tsx
src/shared/i18n/index.ts
```

Main concepts:

- `AppLocale = "en-US" | "zh-CN"`
- `DEFAULT_APP_LOCALE = "en-US"`
- `APP_LOCALE_OPTIONS`
- `normalizeAppLocale(...)`
- `AppI18nProvider`
- `useAppI18n()`
- `formatAppMessage(...)`
- `formatAppMessageById(...)`
- `formatAppMessageByDefault(...)`
- `getActiveAppLocale()`

`AppI18nProvider` wraps the shell in `src/app/App.tsx`. It provides React hooks for components and a small module-level formatting bridge for utility code that is not inside React render scope, such as error/message formatting helpers.

The `zh-CN` dictionary is generated from English defaults with `[待译] ` prepended. For local descriptors created through `td(id, defaultMessage)`, missing `zh-CN` entries also fall back to the placeholder pattern.

## 4. Message Migration

The migration covers the main user-visible UI areas:

- App shell, title bar, sidebar, notice banner.
- Workspace, settings, add/open pack, pack metadata.
- Card list, card edit drawer, card asset controls, card text form, card info form.
- Strings list/browser.
- Import and export wizards.
- Standard pack status, browser, and standard card inspector.
- Shared confirmation/warning dialog.
- Error, warning, validation issue, and job status/stage formatting.

The implementation intentionally keeps these values untranslated:

- User-authored card names and descriptions.
- Pack names, authors, versions, descriptions.
- Pack string values.
- File paths.
- IDs and numeric keys.
- `YGOPro`, `CDB`, `strings.conf`, and similar domain names.

## 5. Backend Config Behavior

Backend config rules now define:

```rust
pub const DEFAULT_APP_LANGUAGE: &str = "en-US";
pub const SUPPORTED_APP_LANGUAGES: [&str; 2] = ["en-US", "zh-CN"];
```

Behavior:

- New default config uses `app_language = "en-US"`.
- Loading existing config normalizes empty or unsupported `app_language` back to `en-US`.
- Saving config validates the submitted value before normalization.
- Saving unsupported values such as `ja-JP` fails with `config.validation_failed`.
- Saving `zh-CN` succeeds.

This preserves compatibility for old local config files while preventing new invalid UI locale values from being saved.

## 6. Card Label Formatting

New frontend-only card label formatter:

```text
src/shared/utils/cardLabels.ts
```

It covers:

- Primary type.
- OT.
- Monster flags.
- Race.
- Attribute.
- Spell subtype.
- Trap subtype.
- Link marker.
- Card category labels.
- Known parts of `subtype_display`.

Important compatibility decision:

- `CardListRow.subtype_display` remains the existing backend-joined string.
- The frontend splits it by `" / "`.
- Known English parts are mapped to i18n message ids.
- Unknown parts are displayed as-is.
- CSS/data behavior remains based on the original English segment, not the localized display string.

This avoids a backend DTO/schema change while still making current subtype labels localizable.

## 7. Formatting and Utility Changes

Timestamp formatting:

- `formatTimestamp` now uses the active UI locale.
- It keeps 24-hour formatting.
- Missing timestamps use localized `common.noRecordedTime`.

Message formatting:

- `src/shared/utils/messages.ts` remains the central mapping entry for backend errors, validation issues, warning codes, and job states.
- It now routes descriptors through the i18n bridge instead of manually interpolating English strings.

Language display:

- Built-in text language labels are displayed through UI i18n.
- Custom language labels remain user-authored and are not translated.

## 8. Dependency Changes

Added runtime dependency:

```json
"react-intl": "^10.1.4"
```

Updated:

```text
package.json
package-lock.json
```

## 9. Validation

Validation run during implementation:

```text
npm.cmd run typecheck
npm.cmd run build
cargo fmt
cargo test --offline
```

Results:

- TypeScript typecheck passed.
- Frontend production build passed.
- Rust formatting completed.
- Rust offline tests passed.

Note:

- `npm.cmd run build` failed inside the sandbox with a Vite `spawn EPERM`, then passed when run outside the sandbox with approval.
- The successful Vite build produced only dependency-level `"use client"` directive warnings from third-party packages.

## 10. Known Limits

### 10.1 Placeholder Translation Only

`zh-CN` currently does not contain real translations. It only proves the architecture and call sites.

Future work:

- Replace placeholders in `src/shared/i18n/messages.ts` with real Simplified Chinese copy.
- Decide whether translations should be maintained manually, generated from extracted descriptors, or managed through an external translation workflow.

### 10.2 Message Catalog Organization

The first slice uses a mixed approach:

- Stable shared ids in `EN_MESSAGES`.
- Local component descriptors through `td(id, defaultMessage)`.

This is pragmatic for an initial migration, but the catalog will need more governance as copy grows.

Future work:

- Decide naming conventions for message ids.
- Move high-traffic local descriptors into the central dictionary.
- Add a script or check to detect duplicate ids and missing defaults.
- Consider extraction tooling if the project grows beyond two locales.

### 10.3 Existing Notices Are Stored as Rendered Strings

Current notices store already-formatted strings. If the user changes UI language while old notices are still visible, old notices do not re-render into the new locale.

Future work:

- Store notice descriptors plus values instead of rendered strings.
- Format notices at render time in `NoticeBanner`.

### 10.4 Some Data-Originated Backend Messages Remain Raw

The app now maps many backend error and warning codes through localized descriptors, but arbitrary backend message strings can still surface as fallback text.

Future work:

- Keep expanding code-to-message mappings.
- Prefer structured error codes and params over user-facing backend prose.
- Audit less common backend errors after more manual testing.

### 10.5 Manual UI QA Still Needed

Automated type/build/test checks passed, but this slice did not include screenshot or full manual UI traversal.

Future work:

- Manually switch to `zh-CN` and inspect all major flows.
- Check compact controls for `[待译] ...` overflow.
- Pay special attention to table headers, modal buttons, wizard steps, and card info chips.

### 10.6 No RTL or Locale-Specific Layout Work

The UI locale model can be extended, but the UI has only been designed for LTR languages.

Future work:

- If Arabic/Hebrew/etc. are ever added, introduce direction metadata per locale.
- Audit layout, icons, sidebars, and directional wording.

### 10.7 No Backend Structured Subtype DTO

The frontend parser for `subtype_display` is intentionally a minimal-change solution.

Future work:

- If subtype display becomes more complex, add structured subtype data to the backend DTO.
- Keep `subtype_display` for compatibility if needed.
- Avoid standard pack schema changes until the structured data has clear product value.

### 10.8 Test Coverage Is Backend-Heavy

The backend config behavior is covered by Rust tests. Frontend i18n is currently covered by typecheck/build only.

Future work:

- Add component tests if a frontend test runner is introduced.
- Test locale switching in Settings.
- Test card subtype/category label rendering under `zh-CN`.
- Test unknown subtype passthrough.

## 11. Suggested Next Steps

Recommended order:

1. Manual smoke test the default English UI.
2. Switch `App language` to `zh-CN` and inspect major screens for `[待译] ...` coverage and layout issues.
3. Replace the most visible `zh-CN` placeholders with real translations.
4. Convert notices from rendered strings to descriptors.
5. Add a message catalog lint/extraction check.
6. Expand backend code-to-message coverage for less common errors.
7. Decide whether `subtype_display` frontend parsing remains sufficient or should become a structured DTO in a later backend change.

## 12. Summary

The first UI i18n slice establishes the architecture and most important call sites without changing pack/card language storage.

The final model is:

```text
Program UI:
GlobalConfig.app_language -> AppI18nProvider -> react-intl messages

Card/pack text:
LanguageCode and text_language_catalog remain the authoring/data model

Card list labels:
Backend subtype_display stays unchanged
Frontend maps known English parts to UI i18n labels
Unknown parts pass through unchanged
```

This gives the project a working i18n foundation while preserving data compatibility and avoiding unnecessary backend schema churn.
