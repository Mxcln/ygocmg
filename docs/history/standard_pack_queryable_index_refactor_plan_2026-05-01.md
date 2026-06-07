# Standard Pack Queryable Index Refactor Plan

Date: 2026-05-01

## Implementation Status

Status as of 2026-05-01:

- Phase 1 is implemented: Standard Pack runtime access is behind a repository boundary, and `list_standard_setnames` is available as a setname-specific command.
- Phase 2 is implemented: rebuild writes `<app_data>/standard_pack/manifest.json` as a lightweight sidecar.
- Phase 3 is implemented: rebuild writes `<app_data>/standard_pack/index.sqlite`, validates row counts, and atomically replaces the previous SQLite cache.
- Phase 4 is implemented: production Standard Pack read paths use SQLite, not `index.json`.
- Phase 5 is implemented: the SQLite cache includes an FTS5 card-search table, and card keyword search has an FTS-backed SQL query path while preserving existing contains behavior.
- Phase 6 is implemented: the full `index.json` writer/reader and `JsonStandardPackRepository` have been removed.
- The production runtime cache no longer stores `StandardPackIndexFile`; it caches SQLite-stamp-bound manifest data, namespace baseline, and setname lists by language.
- Regression verification passed with `cargo fmt`, `cargo test --offline --test standard_pack`, and full `cargo test --offline`.

## 1. Purpose

This document proposes a structural refactor of the Standard Pack backend model.

The current Standard Pack implementation is functionally correct, but its read model is built around a single large JSON snapshot. That makes the first Standard Pack access expensive, and it also lets unrelated workflows, such as opening the first custom-card editor drawer, accidentally pay the full Standard Pack loading cost.

The goal is not to hide latency with local UI workarounds. The goal is to change the Standard Pack architecture so that Standard Pack data is stored and queried as a read-only reference index instead of being loaded as one full in-memory object.

## 2. Current Situation

### 2.1 Runtime state

The application currently keeps these runtime structures in `AppState`:

- `sessions`: workspace and open custom-pack sessions.
- `confirmation_cache`: staged card and strings confirmation entries.
- `preview_token_cache`: import/export preview entries.
- `standard_pack_runtime_cache`: lightweight SQLite-stamp-bound Standard Pack runtime data.

Relevant files:

- `src-tauri/src/bootstrap/app_state.rs`
- `src-tauri/src/runtime/sessions/mod.rs`
- `src-tauri/src/runtime/standard_pack_cache.rs`
- `src-tauri/src/runtime/confirmation_cache.rs`
- `src-tauri/src/runtime/preview_token_cache.rs`

### 2.2 Custom pack model

Custom packs are author-state packs in the workspace. They are stored as pack directories:

- `metadata.json`
- `cards.json`
- `strings.json`
- `pics/`
- `pics/field/`
- `script/`

When a custom pack is opened, `PackService::open_pack` reads the pack files and builds a full `PackSession`.

`PackSession` includes:

- `metadata`
- full `cards`
- full `strings`
- `asset_index`
- `card_list_cache`
- `source_stamp`
- `revision`

This model is acceptable for author-state custom packs because custom packs are opened explicitly, are normally much smaller than the Standard Pack, and need coherent in-memory snapshots for editing.

### 2.3 Standard Pack model

The Standard Pack is different:

- It is global, not workspace-local.
- It is read-only.
- It is rebuilt from an external YGOPro source.
- It is used both by UI browsing and by validation/reference logic.

Before this refactor, the Standard Pack rebuild wrote a single large file:

```text
<app_data>/standard_pack/index.json
```

That file is represented by `StandardPackIndexFile` and currently includes:

- source snapshot
- source language
- indexed timestamp
- all standard cards
- per-card list rows
- per-card asset state
- raw CDB fields
- all standard strings
- strings namespace baseline

On the tested local machine, this file is about 33.7 MB and contains:

- 14,670 cards
- 1,246 strings records

As of Phase 6, rebuild no longer writes `index.json`. Production reads use:

```text
<app_data>/standard_pack/index.sqlite
```

The Phase 2 sidecar remains:

```text
<app_data>/standard_pack/manifest.json
```

Runtime commands do not use `index.json` as a fallback when SQLite is missing. A JSON-only cache is treated as a missing Standard Pack index, and users recover by rebuilding.

### 2.4 Current first-load behavior

Before this refactor, opening Standard Pack triggered:

1. `get_standard_pack_status`
2. `standard_pack::status`
3. `standard_pack::load_index`
4. full read and deserialize of `index.json`

Then the card list triggered:

1. `search_standard_cards`
2. `standard_pack_index_cache.get_or_load`
3. another full `load_index` if the runtime cache is cold
4. full-card filtering, cloning, sorting, and pagination

Opening a custom-card editor drawer also triggered Standard Pack access:

1. `CardEditDrawer` starts a `standard-setnames` query immediately.
2. The query calls `search_standard_strings`.
3. `search_standard_strings` calls `standard_pack_index_cache.get_or_load`.
4. A cold cache loads the whole Standard Pack index even though only setname strings are needed.

## 3. Architectural Diagnosis

The current Standard Pack design conflates several read models into one large snapshot.

Before the refactor, the single `index.json` acted as:

1. Status manifest.
2. Card list index.
3. Card detail store.
4. Strings index.
5. Setname source.
6. Namespace baseline source.
7. Asset state index.

This causes three structural performance problems.

### 3.1 Coarse loading boundary

Any code path that needs any Standard Pack data may have to load all Standard Pack data.

Examples:

- Status needs only metadata, but loads all cards.
- Setname picker needs only setname strings, but loads all cards.
- Namespace validation needs only code/string key baselines, but loads full card details.
- First page browsing needs a few list rows, but loads full card detail for every card.

### 3.2 JSON is the wrong long-term storage shape

JSON is good for author-state pack files and simple cache artifacts. It is not ideal for a large read-only reference index that needs:

- status reads
- indexed lookup by code
- search
- sorting
- pagination
- strings filtering
- baseline extraction
- partial reads

The current JSON format forces whole-file IO and whole-file deserialization before the application can answer small queries.

### 3.3 Runtime cache is too large and too semantic

`StandardPackIndexCache` currently caches the whole Standard Pack object. That makes cache hits fast enough for some repeated operations, but it also makes cold loads expensive and memory-heavy.

Runtime cache should be organized around hot query results and small indexes, not around the entire Standard Pack snapshot.

## 4. Target Architecture

Standard Pack should become a queryable read-only reference database.

The Standard Pack backend should be split into these responsibilities:

### 4.1 `StandardPackSource`

Responsible for discovering and describing the external YGOPro source.

Responsibilities:

- Locate YGOPro root.
- Locate the unique root `.cdb`.
- Read source file stamps.
- Read `strings.conf` stamp.
- Detect source changes.

It should not own runtime browsing queries.

### 4.2 `StandardPackIndexStore`

Responsible for storing rebuilt Standard Pack data in an internal queryable format.

Recommended storage:

```text
<app_data>/standard_pack/index.sqlite
```

The project already depends on `rusqlite`, so SQLite is available without adding a new core storage dependency.

Responsibilities:

- Create schema.
- Migrate schema.
- Replace index atomically after rebuild.
- Open read-only connections for query services.
- Validate schema version.
- Expose lightweight manifest reads.

### 4.3 `StandardPackRepository`

Responsible for read APIs over the internal index.

Suggested interface:

```rust
pub trait StandardPackRepository {
    fn status(&self) -> AppResult<StandardPackStatus>;
    fn search_cards(&self, input: SearchStandardCardsInput) -> AppResult<StandardCardPageDto>;
    fn get_card(&self, code: u32) -> AppResult<StandardCardDetailDto>;
    fn search_strings(&self, input: SearchStandardStringsInput) -> AppResult<StandardStringsPageDto>;
    fn namespace_baseline(&self) -> AppResult<StandardNamespaceBaseline>;
    fn setnames(&self, language: &str) -> AppResult<Vec<StandardSetnameEntry>>;
}
```

The application layer should depend on this query boundary instead of directly calling `load_index`.

### 4.4 `StandardNamespaceService`

Responsible for code and strings namespace checks.

Responsibilities:

- Provide `standard_codes`.
- Provide strings baseline.
- Provide setname base/key baseline.
- Serve custom pack validation, import, export, and card code suggestion.

This service must not require full Standard Pack card details.

### 4.5 `StandardPackRuntimeCache`

Runtime cache should become small and scenario-specific.

Recommended cache entries:

- Manifest/status cache.
- Namespace baseline cache.
- Setname list cache.
- First page cache for default Standard Pack card list.
- LRU cache for recently opened standard card details.

The cache should not hold the full Standard Pack as the primary model.

## 5. Proposed SQLite Schema

The schema should be treated as an internal cache schema. It is rebuildable and may be replaced when the application changes.

### 5.1 Metadata

```sql
create table standard_meta (
  key text primary key,
  value text not null
);
```

Required keys:

- `schema_version`
- `source_language`
- `indexed_at`
- `ygopro_path`
- `cdb_path`
- `cdb_modified`
- `cdb_len`
- `strings_modified`
- `strings_len`
- `card_count`
- `string_count`

Alternative normalized form:

```sql
create table standard_manifest (
  id integer primary key check (id = 1),
  schema_version integer not null,
  source_language text not null,
  indexed_at text not null,
  ygopro_path text not null,
  cdb_path text not null,
  cdb_modified integer,
  cdb_len integer not null,
  strings_modified integer,
  strings_len integer,
  card_count integer not null,
  string_count integer not null
);
```

The normalized form is preferable for typed reads.

### 5.2 Cards

```sql
create table standard_cards (
  code integer primary key,
  alias integer not null,
  ot text not null,
  category integer not null,
  primary_type text not null,
  subtype_display text not null,
  atk integer,
  def integer,
  level integer,
  raw_type integer not null,
  raw_race integer not null,
  raw_attribute integer not null,
  raw_level integer not null,
  detail_json text not null
);
```

`detail_json` stores the complete `EditableCardDto` or `CardEntity` equivalent for single-card detail reads.

The table duplicates list fields intentionally. Runtime list queries should not deserialize full card JSON.

Indexes:

```sql
create index idx_standard_cards_primary_type on standard_cards(primary_type, subtype_display, code);
create index idx_standard_cards_subtype on standard_cards(subtype_display, code);
```

### 5.3 Card texts

```sql
create table standard_card_texts (
  code integer not null,
  language text not null,
  name text not null,
  desc text not null,
  strings_json text not null,
  primary key (code, language)
);
```

Indexes:

```sql
create index idx_standard_card_texts_language_name on standard_card_texts(language, name, code);
```

Optional FTS table:

```sql
create virtual table standard_card_texts_fts using fts5(
  name,
  desc,
  content='standard_card_texts',
  content_rowid='rowid'
);
```

The implemented Phase 5 schema uses a materialized card-search FTS table for source-language card list rows:

```sql
create virtual table standard_card_search_fts using fts5(
  code unindexed,
  language unindexed,
  name,
  card_desc,
  primary_type,
  subtype_display
);
```

The repository still keeps `instr(lower(...), keyword)` predicates in the SQL where clause so existing contains behavior remains compatible for substrings and languages/tokenization that FTS may not segment well.

### 5.4 Card list rows

This table can be either materialized directly or represented by a view joining `standard_cards`, `standard_card_texts`, and `standard_assets`.

Materialized version:

```sql
create table standard_card_list_rows (
  code integer not null,
  language text not null,
  name text not null,
  desc text not null,
  primary_type text not null,
  subtype_display text not null,
  atk integer,
  def integer,
  level integer,
  has_image integer not null,
  has_script integer not null,
  has_field_image integer not null,
  primary key (code, language)
);
```

Indexes:

```sql
create index idx_standard_card_rows_code on standard_card_list_rows(code);
create index idx_standard_card_rows_name on standard_card_list_rows(name, code);
create index idx_standard_card_rows_type on standard_card_list_rows(primary_type, subtype_display, code);
```

The materialized version is recommended because Standard Pack rebuild is already a deliberate job. Precomputing list rows during rebuild moves work out of runtime.

### 5.5 Assets

```sql
create table standard_assets (
  code integer primary key,
  has_image integer not null,
  has_script integer not null,
  has_field_image integer not null
);
```

This can be folded into `standard_card_list_rows` if no independent asset query is needed.

### 5.6 Strings

```sql
create table standard_strings (
  kind text not null,
  key integer not null,
  language text not null,
  value text not null,
  primary key (kind, key, language)
);
```

Indexes:

```sql
create index idx_standard_strings_kind_key on standard_strings(kind, key);
create index idx_standard_strings_kind_value on standard_strings(kind, value, key);
```

This table serves:

- Standard Pack strings browser.
- Setname picker.
- Namespace baseline construction.

### 5.7 Namespace baseline

Option A: derive baseline from `standard_cards` and `standard_strings` queries.

Option B: materialize dedicated baseline tables:

```sql
create table standard_code_baseline (
  code integer primary key
);

create table standard_string_baseline (
  kind text not null,
  key integer not null,
  primary key (kind, key)
);

create table standard_setname_base_baseline (
  base integer primary key
);
```

Option B is recommended. It makes validation paths cheap and explicit.

## 6. Query Behavior

### 6.1 Status

Status should only read:

- global config
- source file metadata
- `standard_manifest`

It should not read standard cards or strings.

Expected behavior:

- If no YGOPro path: return `NotConfigured`.
- If no source language: return `MissingLanguage`.
- If no index DB: return `MissingIndex`.
- If schema mismatch: return `MissingIndex` or `SchemaMismatch`.
- If source stamp differs: return `Stale`.
- Else return `Ready`.

### 6.2 Card list

Default query:

```sql
select code, name, desc, primary_type, subtype_display, atk, def, level,
       has_image, has_script, has_field_image
from standard_card_list_rows
where language = ?
order by code asc
limit ? offset ?;
```

Name sort:

```sql
order by name asc, code asc
```

Type sort:

```sql
order by primary_type asc, subtype_display asc, code asc
```

Keyword search should initially support:

- code exact or prefix match
- name contains
- desc contains
- subtype contains
- primary type label match

Keyword search uses SQL predicates plus the FTS5 table for token matches. Full ranking is still deliberately simple; sort order remains the user-selected code/name/type order.

### 6.3 Card detail

Single card detail should query one row by code:

```sql
select detail_json
from standard_cards
where code = ?;
```

Then deserialize only that one card.

### 6.4 Strings browser

```sql
select kind, key, value
from standard_strings
where language = ?
  and (? is null or kind = ?)
  and (? is null or key = ?)
  and (? is null or value like ?)
order by kind asc, key asc
limit ? offset ?;
```

### 6.5 Setname picker

Setnames should have a dedicated repository method:

```rust
fn setnames(&self, language: &str) -> AppResult<Vec<StandardSetnameEntry>>;
```

It should query only:

```sql
select key, value
from standard_strings
where language = ?
  and kind = 'setname'
order by value asc, key asc;
```

The card editor drawer should never need `search_standard_strings(pageSize: 10000)` as its long-term API.

### 6.6 Namespace baseline

Code baseline:

```sql
select code from standard_code_baseline;
```

String baseline:

```sql
select kind, key from standard_string_baseline;
```

These queries can be cached in memory after first use because their outputs are small compared with full card details.

## 7. Application Layer Changes

### 7.1 New module shape

Recommended backend module organization:

```text
src-tauri/src/
  application/
    standard_pack/
      service.rs
      repository.rs
      dto_mapping.rs
  infrastructure/
    standard_pack/
      mod.rs
      source.rs
      rebuild.rs
      sqlite_store.rs
      legacy_json_store.rs
  runtime/
    standard_pack_cache.rs
```

### 7.2 `application/standard_pack/service.rs`

The application service should stop knowing whether the index is JSON or SQLite.

It should depend on repository functions:

- `get_status`
- `rebuild_index`
- `search_cards`
- `get_card`
- `search_strings`
- `get_setnames`
- `get_namespace_baseline`

### 7.3 `infrastructure/standard_pack/mod.rs`

The current module does too much:

- source discovery
- index loading
- index saving
- status
- rebuild
- asset scan
- strings parsing
- baseline extraction

It should be split so that source, rebuild, and store are separate.

### 7.4 New Standard Pack DTOs

Add a dedicated setname DTO:

```rust
pub struct StandardSetnameEntryDto {
    pub key: u32,
    pub value: String,
}

pub struct ListStandardSetnamesInput {
    pub language: Option<String>,
}
```

Expose a command:

```rust
list_standard_setnames
```

This replaces the editor drawer's use of `search_standard_strings(pageSize: 10000)`.

### 7.5 Existing command compatibility

Keep these commands:

- `get_standard_pack_status`
- `rebuild_standard_pack_index`
- `search_standard_cards`
- `search_standard_strings`
- `get_standard_card`

The command surface can remain stable while the backend storage changes.

Additive command:

- `list_standard_setnames`

## 8. Frontend Changes

### 8.1 Standard Pack view

The Standard Pack view should behave the same from the user's perspective.

Expected frontend behavior after backend refactor:

- Status loads quickly from manifest.
- Card list first page loads without full Standard Pack hydration.
- Card detail loads one card by code.
- Strings tab loads only strings rows.

### 8.2 Card editor drawer

The card editor should use a dedicated setname API:

```ts
standardPackApi.listSetnames({ language })
```

This is not only a UI optimization. It expresses the correct domain boundary:

- The drawer needs setname reference data.
- It does not need the Standard Pack strings browser.
- It does not need full Standard Pack cards.

### 8.3 Query cache

Frontend TanStack Query cache remains useful, but it should not be the primary way to hide backend architecture cost.

Recommended query keys:

- `["standard-pack-status"]`
- `["standard-cards", keyword, sortBy, sortDirection, page]`
- `["standard-card", code]`
- `["standard-strings", filters...]`
- `["standard-setnames", language]`

## 9. Runtime Cache Changes

### 9.1 Current cache replacement

Current:

```rust
StandardPackIndexCache {
    inner: Arc<RwLock<Option<CachedStandardPackIndex>>>
}
```

Target:

```rust
StandardPackRuntimeCache {
    manifest: Option<CachedManifest>,
    namespace_baseline: Option<CachedNamespaceBaseline>,
    setnames_by_language: BTreeMap<LanguageCode, CachedSetnames>,
    card_detail_lru: CardDetailLru,
    first_pages: BTreeMap<StandardPageCacheKey, CachedCardPage>,
}
```

The exact implementation can be simpler at first. The important change is that the cache no longer treats the full Standard Pack as the unit of caching.

### 9.2 Cache invalidation

All Standard Pack runtime caches should be invalidated when:

- `rebuild_standard_pack_index` succeeds.
- Standard index DB file stamp changes.
- Global config changes `ygopro_path`.
- Global config changes `standard_pack_source_language`.
- Schema version changes.

### 9.3 DB connection handling

For Tauri command calls, a simple per-call read-only SQLite connection is acceptable initially.

If repeated open cost becomes measurable, introduce:

- a shared read-only connection behind a lock, or
- a small connection manager.

Because the Standard Pack DB is read-only during normal browsing, concurrency risk is low.

## 10. Rebuild Flow

### 10.1 Current rebuild flow

Current rebuild:

1. Discover source.
2. Load CDB records.
3. Scan assets.
4. Derive card list rows.
5. Load strings.
6. Build one `StandardPackIndexFile`.
7. Save the queryable SQLite cache.
8. Refresh runtime cache.

### 10.2 Target rebuild flow

Target rebuild:

1. Discover source.
2. Create temp SQLite DB under `standard_pack/`.
3. Apply schema.
4. Load CDB cards.
5. Scan assets once.
6. Insert card rows and text rows in a transaction.
7. Insert asset rows.
8. Load strings and insert string rows.
9. Insert namespace baseline rows.
10. Insert manifest row.
11. Validate row counts.
12. Atomically replace old index DB.
13. Clear runtime caches.

Implemented final behavior:

- `save_index()` writes SQLite first, then writes `manifest.json`.
- SQLite is written to a temp DB path, validated, closed, and then installed with a backup-rename replacement flow that works on Windows.
- If SQLite creation, insertion, validation, or replacement fails, rebuild fails and the existing SQLite file is preserved where possible.
- After successful rebuild, the lightweight runtime cache is cleared instead of being populated with a full `StandardPackIndexFile`.

### 10.3 Atomic replacement

The rebuild job should write to:

```text
index.sqlite.tmp
```

Then replace:

```text
index.sqlite
```

The replacement should be performed only after the new DB validates successfully.

### 10.4 Optional compatibility output

The Phase 6 implementation removed the `index.json` writer. The remaining JSON artifact is the small `manifest.json` sidecar.

## 11. Migration Strategy

### Phase 1: Introduce repository boundary

Status: Implemented on 2026-05-01.

Goal:

- Stop application code from depending directly on `load_index`.
- Keep existing JSON storage internally.

Tasks:

- [x] Add `StandardPackRepository` abstraction.
- [x] Implement initial `JsonStandardPackRepository` using the then-current `StandardPackIndexCache`.
- [x] Move `get_status`, `search_cards`, `get_card`, `search_strings`, baseline access behind repository methods.
- [x] Add `list_standard_setnames` command, implemented from current JSON index for now.

Expected result:

- No storage behavior change yet.
- Clear seam for SQLite migration.
- Card editor can depend on a setname-specific API.

Implemented notes:

- Existing command contracts remain compatible.
- `list_standard_setnames` is additive and returns only `{ key, value }` setname entries.
- Card editor setname lookup uses the setname-specific API instead of `search_standard_strings(pageSize: 10000)`.
- Standard Pack status still read the legacy JSON index in this phase; later phases replaced that path with SQLite status.

### Phase 2: Add lightweight manifest

Status: Implemented on 2026-05-01.

Goal:

- Make status independent from full Standard Pack card data.

Tasks:

- [x] Write `manifest.json` during rebuild.
- [x] Store manifest schema version, SQLite schema version, source snapshot, source language, indexed timestamp, card count, and string count.
- [x] Change low-level status metadata loading to prefer SQLite manifest/status data.
- [x] Remove fallback to old `index.json` in the final read path.
- [x] Avoid auto-writing manifest on status reads.

Expected result:

- Opening Standard Pack no longer requires full index read just to determine status.

Implemented notes:

- The manifest path is `<app_data>/standard_pack/manifest.json`.
- Manifest schema version is independent from `STANDARD_SQLITE_SCHEMA_VERSION`.
- Manifest failures make rebuild fail, because a successful rebuild should leave the lightweight sidecar ready.
- Production status reads the SQLite `standard_manifest`; the JSON manifest is a small sidecar/debug artifact, not the runtime source of truth.

### Phase 3: Add SQLite index writer

Status: Implemented on 2026-05-01.

Goal:

- Rebuild can produce `index.sqlite`.

Tasks:

- [x] Add `sqlite_store.rs`.
- [x] Define `STANDARD_SQLITE_SCHEMA_VERSION`.
- [x] Implement schema creation.
- [x] Implement bulk insert transaction.
- [x] Insert manifest, cards, card texts, source-language list rows, assets, strings, code baseline, string baseline, and setname base baseline.
- [x] Add row-count validation.
- [x] Use temp DB plus validation plus atomic replacement.
- [x] Keep JSON writer/reader available during development.

Expected result:

- Rebuild writes queryable index.
- Existing UI can still use old JSON path until repository switches.

Implemented notes:

- The SQLite path is `<app_data>/standard_pack/index.sqlite`.
- The current SQLite schema version is `2`.
- `detail_json` stores serialized `CardEntity`.
- `strings_json` stores serialized per-card text strings.
- `standard_strings` stores every language value present in the in-memory index.
- `standard_manifest.string_count` remains the number of Standard Pack string records, not the number of language-value rows.

### Phase 4: Switch Standard Pack reads to SQLite

Status: Implemented on 2026-05-01.

Goal:

- Runtime Standard Pack browsing no longer loads full JSON.

Tasks:

- [x] Implement `SqliteStandardPackRepository`.
- [x] Route `get_status`, `search_cards`, `get_card`, `search_strings`, `list_standard_setnames`, and baseline calls through SQLite.
- [x] Switch Standard Pack consumers in card, import, export, and pack write services from `JsonStandardPackRepository` to `SqliteStandardPackRepository`.
- [x] Replace `StandardPackIndexCache` with `StandardPackRuntimeCache` so production caching no longer stores a full `StandardPackIndexFile`.
- [x] Add runtime caches for SQLite manifest, setnames by language, and namespace baseline.
- [x] Treat JSON-only cache as missing index in production status.
- [x] Keep public Tauri command and TypeScript contracts unchanged.

Expected result:

- Standard Pack first open becomes a small manifest read plus first-page query.
- Card detail reads one card.
- Setname picker reads only setname rows.
- Strings browser reads only strings rows.

Implemented notes:

- `get_standard_pack_status` opens SQLite and reads `standard_manifest`; missing `index.sqlite` maps to missing index/rebuild required.
- Card list queries use SQL count/order/page and a keyword where clause with FTS plus contains-compatible predicates.
- `get_standard_card` queries one `standard_cards.detail_json` row and asset booleans.
- `namespace_baseline()` reads baseline tables and does not deserialize card details.
- `JsonStandardPackRepository` has been removed.

### Phase 5: Optimize search

Status: Implemented on 2026-05-01.

Goal:

- Improve keyword search for large standard card sets.

Tasks:

- [x] Add FTS5 table for card name, desc, primary type, and subtype.
- [x] Add FTS query path for non-empty keyword.
- [x] Keep simple code/text contains handling for backward-compatible search behavior.
- [x] Keep stable user-selected sort order for code/name/type.

Expected result:

- Standard Pack search remains fast as data grows.

Implemented notes:

- `standard_card_search_fts` is populated during SQLite index writing and row-count validated.
- Keyword card search now uses SQL count/order/limit/offset instead of loading all list rows into Rust first.
- FTS is used as an additional match path. Contains predicates remain in the same SQL where clause to preserve current behavior for substrings and non-FTS-friendly text.

### Phase 6: Remove legacy JSON full index

Status: Implemented on 2026-05-01.

Goal:

- Eliminate duplicate storage and full JSON load path.

Tasks:

- [x] Delete `index.json` writer.
- [x] Remove `JsonStandardPackRepository`.
- [x] Remove full-index JSON loader/status fallback helpers.
- [x] Remove full-index runtime cache and rename it to `StandardPackRuntimeCache`.
- [x] Keep migration error messages clear: users can rebuild the Standard Pack index if old cache files are present.

Expected result:

- Standard Pack architecture is fully queryable and no longer whole-file oriented.

Implemented notes:

- `save_index()` no longer writes `<app_data>/standard_pack/index.json`.
- `standard_pack::status()` now reads SQLite metadata rather than JSON/manifest stamp metadata.
- A JSON-only cache is reported as a missing SQLite index, which drives users toward rebuild.
- `StandardPackIndexFile` remains only as an in-memory rebuild bundle passed to the SQLite writer.

## 12. Testing Plan

### 12.1 Unit tests

Add tests for:

- Manifest read/write.
- Schema version mismatch.
- Source stamp comparison.
- Card row insertion.
- Card detail lookup by code.
- Strings filtering by kind/key/keyword.
- Setname listing.
- Namespace baseline construction.

### 12.2 Integration tests

Extend `src-tauri/tests/standard_pack.rs`:

- [x] Rebuild writes SQLite index.
- [x] Status works without reading full card data.
- [x] JSON-only cache is missing index for production SQLite status.
- [x] Search cards returns expected page and total.
- [x] Card keyword search uses the FTS-backed SQL path.
- [x] Get standard card returns detail by code.
- [x] Missing standard card returns `standard_pack.card_not_found`.
- [x] Search strings returns expected filtered records.
- [x] Setnames can be listed without card data access.
- [x] Namespace baseline can be built without card detail data.
- [x] Rebuild invalidates runtime caches.
- [x] Stale detection works when source CDB stamp changes.
- [x] Schema mismatch asks for rebuild.

### 12.3 Performance tests

Add lightweight benchmark-style tests or diagnostics for local validation:

- Time to read status.
- Time to first standard cards page.
- Time to get one standard card detail.
- Time to list setnames.
- Time to build namespace baseline.

These do not need to be strict CI benchmarks, but they should be easy to run manually.

### 12.4 Regression tests

Preserve existing behavior:

- Standard Pack remains read-only.
- Standard Pack does not become an open custom pack.
- Standard Pack does not write workspace session state.
- Custom pack card editing still validates against Standard Pack code baseline.
- Export/import preview still checks Standard Pack conflicts.

## 13. Acceptance Criteria

The refactor is successful when:

1. `get_standard_pack_status` does not deserialize all standard cards.
2. First Standard Pack card page does not require loading all card details.
3. `get_standard_card` reads only one card detail by code.
4. `search_standard_strings` does not load card data.
5. Card editor setname reference data does not load card data.
6. Standard namespace validation does not load card details.
7. Runtime cache no longer stores a full `StandardPackIndexFile` as its central unit.
8. Rebuild still produces a fully deterministic Standard Pack reference index.
9. Existing Tauri command contracts remain compatible, except for additive commands.
10. Users can recover from old or missing index files by rebuilding the Standard Pack index.
11. Rebuild no longer writes the legacy full `index.json` cache.
12. `JsonStandardPackRepository` and full JSON load paths are removed.

## 14. Expected Performance Outcome

The target model changes cold-load cost from:

```text
read 33.7 MB JSON
deserialize all cards and strings
build code map
filter/sort all rows
return first page
```

to:

```text
open SQLite index
read manifest
query first page with limit/offset
deserialize only returned DTO rows
```

The most important improvement is not just lower latency. The important improvement is that latency scales with the requested operation, not with total Standard Pack size.

Expected qualitative outcomes:

- Standard Pack status should feel immediate.
- First Standard Pack card page should be limited by one indexed query.
- Opening a custom card editor should not pay Standard Pack full-load cost.
- Setname picker should be cheap and stable.
- Standard Pack memory usage should drop because full card details are not globally hydrated.

## 15. Risks and Mitigations

### Risk: SQLite schema becomes too complex

Mitigation:

- Keep `detail_json` for full card detail instead of fully normalizing every card field.
- Normalize only fields needed for search, sort, pagination, and baseline checks.

### Risk: Rebuild implementation becomes harder to reason about

Mitigation:

- Keep rebuild as a single transaction.
- Validate counts before replacing the old DB.
- Keep source discovery and row conversion pure where possible.

### Risk: Search behavior changes

Mitigation:

- Preserve current simple contains behavior first.
- Add FTS as a later phase.
- Add tests for representative name, desc, code, and type searches.

### Risk: Old cache files conflict with new index files

Mitigation:

- Treat Standard Pack index as rebuildable.
- If schema is old or missing, return `MissingIndex` or `SchemaMismatch`.
- Keep UI rebuild flow unchanged.

### Risk: Runtime cache invalidation misses a source change

Mitigation:

- Tie all caches to manifest/source stamp.
- Clear caches after rebuild.
- Clear or mark stale when config changes `ygopro_path` or source language.

## 16. Non-Goals

This refactor does not aim to:

- Make Standard Pack editable.
- Merge Standard Pack into workspace sessions.
- Change custom pack author-state storage.
- Replace custom pack JSON files with SQLite.
- Automatically rebuild Standard Pack when source files change.
- Implement full advanced search ranking in the first phase.

## 17. Recommended First Implementation Slice

The first slice should be architectural but still small enough to review.

Recommended scope:

1. Add repository abstraction.
2. Add setname-specific backend command.
3. Move current JSON Standard Pack reads behind the repository.
4. Change `get_status` to use a lightweight manifest if available.
5. Keep all existing UI behavior.

This slice does not yet require changing the persistent storage to SQLite, but it creates the boundary that makes the SQLite migration clean.

The second slice should introduce SQLite writing during rebuild.

The third slice should switch reads to SQLite and remove full-index runtime caching from hot paths.

## 18. Final Architecture Principle

Custom packs are author-state documents.

Standard Pack is a read-only reference index.

Those two models should not share the same loading strategy. A custom pack can be opened as an editable snapshot. The Standard Pack should be queried like a database.
