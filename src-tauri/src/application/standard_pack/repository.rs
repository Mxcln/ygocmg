use std::collections::BTreeSet;

use rusqlite::types::Value;
use rusqlite::{Connection, OptionalExtension, params, params_from_iter};
use serde::{Serialize, de::DeserializeOwned};

use crate::application::dto::card::{CardListRowDto, SortDirectionDto};
use crate::application::dto::standard_pack::{
    CardFilterMatchModeDto, GetStandardCardInput, ListStandardSetnamesInput, NumericRangeFilterDto,
    SearchStandardCardsInput, SearchStandardStringsInput, SetcodeFilterModeDto,
    StandardCardDetailDto, StandardCardPageDto, StandardCardSearchFiltersDto,
    StandardCardSortFieldDto, StandardPackIndexStateDto, StandardPackStatusDto,
    StandardSetnameEntryDto, StandardStringSortFieldDto, StandardStringsPageDto,
};
use crate::bootstrap::AppState;
use crate::domain::card::model::CardEntity;
use crate::domain::common::error::{AppError, AppResult};
use crate::domain::namespace::model::{StandardNamespaceBaseline, StandardStringNamespaceBaseline};
use crate::domain::resource::model::CardAssetState;
use crate::domain::strings::model::{PackStringEntry, PackStringKind};
use crate::infrastructure::json_store;
use crate::infrastructure::standard_pack;
use crate::infrastructure::standard_pack::sqlite_store::{
    self, StandardPackSqliteManifest, StandardSetnameRecord,
};

pub trait StandardPackRepository {
    fn status(&self) -> StandardPackStatusDto;
    fn search_cards(&self, input: SearchStandardCardsInput) -> AppResult<StandardCardPageDto>;
    fn get_card(&self, input: GetStandardCardInput) -> AppResult<StandardCardDetailDto>;
    fn search_strings(
        &self,
        input: SearchStandardStringsInput,
    ) -> AppResult<StandardStringsPageDto>;
    fn list_setnames(
        &self,
        input: ListStandardSetnamesInput,
    ) -> AppResult<Vec<StandardSetnameEntryDto>>;
    fn namespace_baseline(&self) -> AppResult<StandardNamespaceBaseline>;
    fn strings_baseline(&self) -> AppResult<StandardStringNamespaceBaseline>;
}

pub struct SqliteStandardPackRepository<'a> {
    state: &'a AppState,
}

impl<'a> SqliteStandardPackRepository<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    fn load_manifest(&self) -> AppResult<StandardPackSqliteManifest> {
        self.state
            .standard_pack_runtime_cache
            .manifest(self.state.app_data_dir(), || {
                sqlite_store::load_sqlite_manifest_from_app_data(self.state.app_data_dir())
            })
    }

    fn open_connection(&self) -> AppResult<Connection> {
        sqlite_store::open_readonly(self.state.app_data_dir())
    }
}

impl StandardPackRepository for SqliteStandardPackRepository<'_> {
    fn status(&self) -> StandardPackStatusDto {
        let config = json_store::load_global_config(self.state.app_data_dir()).ok();
        let ygopro_path = config
            .as_ref()
            .and_then(|config| config.ygopro_path.as_deref());
        let configured_source_language = config
            .as_ref()
            .and_then(|config| config.standard_pack_source_language.as_deref());

        let configured = ygopro_path.is_some();
        let source_result = ygopro_path.map(standard_pack::discover_source);
        let source = source_result
            .as_ref()
            .and_then(|result| result.as_ref().ok());
        let source_error = source_result
            .as_ref()
            .and_then(|result| result.as_ref().err())
            .map(|error| error.message.clone());

        let index_result = self.load_manifest();
        let index_error = index_result.as_ref().err().cloned();
        let schema_mismatch = index_error
            .as_ref()
            .is_some_and(|error| error.code == "standard_pack.sqlite_schema_mismatch");
        let sqlite_missing = index_error
            .as_ref()
            .is_some_and(|error| error.code == "standard_pack.sqlite_missing");
        let index = index_result.ok();
        let stale = match (&index, source) {
            (Some(index), Some(source)) => {
                index.source != source.snapshot
                    || configured_source_language
                        .is_some_and(|language| language != index.source_language)
            }
            _ => false,
        };
        let message = if configured && configured_source_language.is_none() {
            Some("standard pack source language is not configured".to_string())
        } else if schema_mismatch {
            Some("standard pack sqlite schema is outdated; rebuild required".to_string())
        } else if sqlite_missing {
            Some("standard pack sqlite index is missing; rebuild required".to_string())
        } else if let Some(error) = index_error {
            Some(error.message)
        } else {
            source_error
        };

        StandardPackStatusDto::from(standard_pack::StandardPackStatus {
            configured,
            source_language_configured: configured_source_language.is_some(),
            ygopro_path: ygopro_path.map(std::path::Path::to_path_buf),
            cdb_path: source.map(|source| source.cdb_path.clone()),
            index_exists: index.is_some(),
            schema_mismatch,
            stale,
            source_language: index.as_ref().map(|index| index.source_language.clone()),
            indexed_at: index.as_ref().map(|index| index.indexed_at),
            card_count: index.as_ref().map(|index| index.card_count).unwrap_or(0),
            message,
        })
    }

    fn search_cards(&self, input: SearchStandardCardsInput) -> AppResult<StandardCardPageDto> {
        let manifest = self.load_manifest()?;
        let connection = self.open_connection()?;
        let keyword = input.keyword.unwrap_or_default().trim().to_lowercase();
        let filters = input
            .filters
            .map(normalize_card_search_filters)
            .transpose()?;
        let page_size = input.page_size.max(1);
        let page = input.page.max(1);
        let start = ((page - 1) as usize).saturating_mul(page_size as usize);
        let (total, items) = load_card_list_page(
            &connection,
            &manifest.source_language,
            &keyword,
            filters.as_ref(),
            &input.sort_by,
            &input.sort_direction,
            page_size,
            start,
        )?;

        Ok(StandardCardPageDto {
            items,
            page,
            page_size,
            total,
            ygopro_path: Some(manifest.source.ygopro_path.clone()),
            revision: revision_from_manifest(&manifest),
        })
    }

    fn get_card(&self, input: GetStandardCardInput) -> AppResult<StandardCardDetailDto> {
        let manifest = self.load_manifest()?;
        let connection = self.open_connection()?;
        let (detail_json, has_image, has_script, has_field_image) = connection
            .query_row(
                "select c.detail_json,
                        coalesce(a.has_image, 0),
                        coalesce(a.has_script, 0),
                        coalesce(a.has_field_image, 0)
                 from standard_cards c
                 left join standard_assets a on a.code = c.code
                 where c.code = ?1",
                params![input.code as i64],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                },
            )
            .optional()
            .map_err(|source| {
                sqlite_query_error("standard_pack.sqlite_get_card_failed", source)
                    .with_detail("code", input.code)
            })?
            .ok_or_else(|| {
                AppError::new(
                    "standard_pack.card_not_found",
                    "standard card was not found",
                )
                .with_detail("code", input.code)
            })?;
        let card: CardEntity = serde_json::from_str(&detail_json).map_err(|source| {
            AppError::new(
                "standard_pack.sqlite_card_detail_deserialize_failed",
                source.to_string(),
            )
            .with_detail("code", input.code)
        })?;

        Ok(StandardCardDetailDto {
            card: card.into(),
            asset_state: CardAssetState {
                has_image: sqlite_bool(has_image),
                has_script: sqlite_bool(has_script),
                has_field_image: sqlite_bool(has_field_image),
            },
            available_languages: vec![manifest.source_language.clone()],
            ygopro_path: manifest.source.ygopro_path.clone(),
        })
    }

    fn search_strings(
        &self,
        input: SearchStandardStringsInput,
    ) -> AppResult<StandardStringsPageDto> {
        let manifest = self.load_manifest()?;
        let connection = self.open_connection()?;
        let keyword = input.keyword.unwrap_or_default().trim().to_lowercase();
        let mut rows = load_string_entries(&connection, &manifest.source_language)?
            .into_iter()
            .filter(|record| {
                input
                    .kind_filter
                    .as_ref()
                    .map_or(true, |kind| &record.kind == kind)
            })
            .filter(|record| input.key_filter.map_or(true, |key| record.key == key))
            .filter(|record| {
                keyword.is_empty()
                    || record.value.to_lowercase().contains(&keyword)
                    || record.key.to_string().contains(&keyword)
                    || format_string_key_hex(record.key)
                        .to_lowercase()
                        .contains(&keyword)
                    || format!("{:?}", record.kind)
                        .to_lowercase()
                        .contains(&keyword)
            })
            .collect::<Vec<_>>();

        match input.sort_by {
            StandardStringSortFieldDto::Kind => rows.sort_by(|left, right| {
                left.kind
                    .cmp(&right.kind)
                    .then(left.key.cmp(&right.key))
                    .then(left.value.cmp(&right.value))
            }),
            StandardStringSortFieldDto::Key => rows.sort_by(|left, right| {
                left.key
                    .cmp(&right.key)
                    .then(left.kind.cmp(&right.kind))
                    .then(left.value.cmp(&right.value))
            }),
            StandardStringSortFieldDto::Value => rows.sort_by(|left, right| {
                left.value
                    .cmp(&right.value)
                    .then(left.kind.cmp(&right.kind))
                    .then(left.key.cmp(&right.key))
            }),
        }

        if matches!(input.sort_direction, SortDirectionDto::Desc) {
            rows.reverse();
        }

        let page_size = input.page_size.max(1);
        let page = input.page.max(1);
        let total = rows.len() as u64;
        let start = ((page - 1) as usize).saturating_mul(page_size as usize);
        let items = if start >= rows.len() {
            Vec::new()
        } else {
            rows.into_iter()
                .skip(start)
                .take(page_size as usize)
                .map(Into::into)
                .collect()
        };

        Ok(StandardStringsPageDto {
            language: manifest.source_language.clone(),
            items,
            page,
            page_size,
            total,
            revision: revision_from_manifest(&manifest),
        })
    }

    fn list_setnames(
        &self,
        input: ListStandardSetnamesInput,
    ) -> AppResult<Vec<StandardSetnameEntryDto>> {
        let manifest = self.load_manifest()?;
        let language = input
            .language
            .unwrap_or_else(|| manifest.source_language.clone());
        let records = self.state.standard_pack_runtime_cache.setnames(
            self.state.app_data_dir(),
            &language,
            || load_setnames_from_sqlite(self.state.app_data_dir(), &language),
        )?;
        Ok(records
            .into_iter()
            .map(|record| StandardSetnameEntryDto {
                key: record.key,
                value: record.value,
            })
            .collect())
    }

    fn namespace_baseline(&self) -> AppResult<StandardNamespaceBaseline> {
        self.state
            .standard_pack_runtime_cache
            .namespace_baseline(self.state.app_data_dir(), || {
                load_namespace_baseline_from_sqlite(self.state.app_data_dir())
            })
    }

    fn strings_baseline(&self) -> AppResult<StandardStringNamespaceBaseline> {
        Ok(self.namespace_baseline()?.strings)
    }
}

#[derive(Debug)]
struct RawCardListRow {
    code: i64,
    name: String,
    desc: String,
    primary_type: String,
    subtype_display: String,
    atk: Option<i32>,
    def: Option<i32>,
    level: Option<i32>,
    has_image: i64,
    has_script: i64,
    has_field_image: i64,
}

#[derive(Debug)]
struct RawStringEntry {
    kind: String,
    key: i64,
    value: String,
}

#[derive(Debug, Clone, Default)]
struct NormalizedStandardCardSearchFilters {
    codes: Vec<u32>,
    code_range: Option<NumericRangeFilterDto>,
    aliases: Vec<u32>,
    alias_range: Option<NumericRangeFilterDto>,
    ots: Vec<crate::domain::card::model::Ot>,
    name_contains: Option<String>,
    desc_contains: Option<String>,
    primary_types: Vec<crate::domain::card::model::PrimaryType>,
    races: Vec<crate::domain::card::model::Race>,
    attributes: Vec<crate::domain::card::model::Attribute>,
    monster_flags: Vec<crate::domain::card::model::MonsterFlag>,
    monster_flag_match: CardFilterMatchModeDto,
    spell_subtypes: Vec<crate::domain::card::model::SpellSubtype>,
    trap_subtypes: Vec<crate::domain::card::model::TrapSubtype>,
    pendulum_left_scale: Option<NumericRangeFilterDto>,
    pendulum_right_scale: Option<NumericRangeFilterDto>,
    link_markers: Vec<crate::domain::card::model::LinkMarker>,
    link_marker_match: CardFilterMatchModeDto,
    setcodes: Vec<u16>,
    setcode_mode: SetcodeFilterModeDto,
    setcode_match: CardFilterMatchModeDto,
    category_masks: Vec<u64>,
    category_match: CardFilterMatchModeDto,
    atk: Option<NumericRangeFilterDto>,
    def: Option<NumericRangeFilterDto>,
    level: Option<NumericRangeFilterDto>,
}

#[derive(Debug, Clone)]
struct StandardCardSqlQuery {
    clauses: Vec<String>,
    params: Vec<Value>,
}

impl StandardCardSqlQuery {
    fn new(language: &str) -> Self {
        Self {
            clauses: vec!["r.language = ?".to_string()],
            params: vec![Value::Text(language.to_string())],
        }
    }

    fn push_clause(&mut self, clause: impl Into<String>) {
        self.clauses.push(clause.into());
    }

    fn push_value(&mut self, value: Value) {
        self.params.push(value);
    }

    fn push_values(&mut self, values: impl IntoIterator<Item = Value>) {
        self.params.extend(values);
    }

    fn where_sql(&self) -> String {
        self.clauses.join(" and ")
    }

    fn count_sql(&self) -> String {
        format!(
            "select count(*)
             from standard_card_list_rows r
             join standard_cards c on c.code = r.code
             where {}",
            self.where_sql()
        )
    }

    fn page_sql(&self, order_by: &str) -> String {
        format!(
            "select r.code, r.name, r.desc, r.primary_type, r.subtype_display,
                    r.atk, r.def, r.level,
                    r.has_image, r.has_script, r.has_field_image
             from standard_card_list_rows r
             join standard_cards c on c.code = r.code
             where {}
             order by {order_by}
             limit ? offset ?",
            self.where_sql()
        )
    }
}

fn normalize_card_search_filters(
    filters: StandardCardSearchFiltersDto,
) -> AppResult<NormalizedStandardCardSearchFilters> {
    let category_masks = filters
        .category_masks
        .unwrap_or_default()
        .into_iter()
        .filter(|value| *value != 0)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    for mask in &category_masks {
        let _ = u64_to_i64_param("category_mask", *mask)?;
    }

    Ok(NormalizedStandardCardSearchFilters {
        codes: unique_u32(filters.codes),
        code_range: normalize_range(filters.code_range),
        aliases: unique_u32(filters.aliases),
        alias_range: normalize_range(filters.alias_range),
        ots: unique_values(filters.ots),
        name_contains: normalize_contains(filters.name_contains),
        desc_contains: normalize_contains(filters.desc_contains),
        primary_types: unique_values(filters.primary_types),
        races: unique_values(filters.races),
        attributes: unique_values(filters.attributes),
        monster_flags: unique_values(filters.monster_flags),
        monster_flag_match: filters.monster_flag_match.unwrap_or_default(),
        spell_subtypes: unique_values(filters.spell_subtypes),
        trap_subtypes: unique_values(filters.trap_subtypes),
        pendulum_left_scale: normalize_range(filters.pendulum_left_scale),
        pendulum_right_scale: normalize_range(filters.pendulum_right_scale),
        link_markers: unique_values(filters.link_markers),
        link_marker_match: filters.link_marker_match.unwrap_or_default(),
        setcodes: filters
            .setcodes
            .unwrap_or_default()
            .into_iter()
            .filter(|value| *value != 0)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        setcode_mode: filters.setcode_mode.unwrap_or_default(),
        setcode_match: filters.setcode_match.unwrap_or_default(),
        category_masks,
        category_match: filters.category_match.unwrap_or_default(),
        atk: normalize_range(filters.atk),
        def: normalize_range(filters.def),
        level: normalize_range(filters.level),
    })
}

fn build_card_sql_query(
    language: &str,
    keyword: &str,
    filters: Option<&NormalizedStandardCardSearchFilters>,
) -> AppResult<StandardCardSqlQuery> {
    let mut query = StandardCardSqlQuery::new(language);
    apply_keyword_clause(&mut query, keyword);
    if let Some(filters) = filters {
        apply_card_filters(&mut query, filters)?;
    }
    Ok(query)
}

fn apply_keyword_clause(query: &mut StandardCardSqlQuery, keyword: &str) {
    if keyword.is_empty() {
        return;
    }
    let fts_query = build_fts_query(keyword);
    query.push_clause(
        "(instr(cast(r.code as text), ?) > 0
          or instr(lower(r.name), ?) > 0
          or instr(lower(r.desc), ?) > 0
          or instr(lower(r.subtype_display), ?) > 0
          or instr(lower(r.primary_type), ?) > 0
          or exists (
              select 1
              from standard_card_search_fts
              where standard_card_search_fts.language = r.language
                and standard_card_search_fts.code = cast(r.code as text)
                and standard_card_search_fts match ?
          ))",
    );
    for _ in 0..5 {
        query.push_value(Value::Text(keyword.to_string()));
    }
    query.push_value(Value::Text(fts_query));
}

fn apply_card_filters(
    query: &mut StandardCardSqlQuery,
    filters: &NormalizedStandardCardSearchFilters,
) -> AppResult<()> {
    add_integer_in_clause(
        query,
        "c.code",
        filters.codes.iter().map(|value| *value as i64).collect(),
    );
    add_range_clause(query, "c.code", filters.code_range.as_ref());
    add_integer_in_clause(
        query,
        "c.alias",
        filters.aliases.iter().map(|value| *value as i64).collect(),
    );
    add_range_clause(query, "c.alias", filters.alias_range.as_ref());
    add_enum_in_clause(query, "c.ot", &filters.ots)?;
    add_contains_clause(query, "r.name", filters.name_contains.as_deref());
    add_contains_clause(query, "r.desc", filters.desc_contains.as_deref());
    add_enum_in_clause(query, "c.primary_type", &filters.primary_types)?;
    add_enum_in_clause(query, "c.race", &filters.races)?;
    add_enum_in_clause(query, "c.attribute", &filters.attributes)?;
    add_match_table_clause(
        query,
        "standard_card_monster_flags",
        "flag",
        &filters.monster_flags,
        &filters.monster_flag_match,
    )?;
    add_enum_in_clause(query, "c.spell_subtype", &filters.spell_subtypes)?;
    add_enum_in_clause(query, "c.trap_subtype", &filters.trap_subtypes)?;
    add_pendulum_clause(
        query,
        filters.pendulum_left_scale.as_ref(),
        filters.pendulum_right_scale.as_ref(),
    );
    add_match_table_clause(
        query,
        "standard_card_link_markers",
        "marker",
        &filters.link_markers,
        &filters.link_marker_match,
    )?;
    add_setcode_clause(query, filters);
    add_category_clause(query, filters)?;
    add_range_clause(query, "c.atk", filters.atk.as_ref());
    add_range_clause(query, "c.def", filters.def.as_ref());
    add_range_clause(query, "c.level", filters.level.as_ref());
    Ok(())
}

fn add_contains_clause(query: &mut StandardCardSqlQuery, column: &str, value: Option<&str>) {
    if let Some(value) = value {
        query.push_clause(format!("instr(lower({column}), ?) > 0"));
        query.push_value(Value::Text(value.to_string()));
    }
}

fn add_integer_in_clause(query: &mut StandardCardSqlQuery, column: &str, values: Vec<i64>) {
    if values.is_empty() {
        return;
    }
    query.push_clause(format!("{column} in ({})", placeholders(values.len())));
    query.push_values(values.into_iter().map(Value::Integer));
}

fn add_enum_in_clause<T: Serialize>(
    query: &mut StandardCardSqlQuery,
    column: &str,
    values: &[T],
) -> AppResult<()> {
    if values.is_empty() {
        return Ok(());
    }
    query.push_clause(format!("{column} in ({})", placeholders(values.len())));
    let params = values
        .iter()
        .map(|value| enum_text_value(column, value).map(Value::Text))
        .collect::<AppResult<Vec<_>>>()?;
    query.push_values(params);
    Ok(())
}

fn add_range_clause(
    query: &mut StandardCardSqlQuery,
    column: &str,
    range: Option<&NumericRangeFilterDto>,
) {
    let Some(range) = range else {
        return;
    };
    if let Some(min) = range.min {
        query.push_clause(format!("{column} >= ?"));
        query.push_value(Value::Integer(min));
    }
    if let Some(max) = range.max {
        query.push_clause(format!("{column} <= ?"));
        query.push_value(Value::Integer(max));
    }
}

fn add_match_table_clause<T: Serialize>(
    query: &mut StandardCardSqlQuery,
    table: &str,
    column: &str,
    values: &[T],
    match_mode: &CardFilterMatchModeDto,
) -> AppResult<()> {
    if values.is_empty() {
        return Ok(());
    }
    let params = values
        .iter()
        .map(|value| enum_text_value(column, value))
        .collect::<AppResult<Vec<_>>>()?;
    match match_mode {
        CardFilterMatchModeDto::Any => {
            query.push_clause(format!(
                "exists (
                    select 1
                    from {table} ft
                    where ft.code = r.code
                      and ft.{column} in ({})
                )",
                placeholders(params.len())
            ));
            query.push_values(params.into_iter().map(Value::Text));
        }
        CardFilterMatchModeDto::All => {
            query.push_clause(format!(
                "r.code in (
                    select code
                    from {table}
                    where {column} in ({})
                    group by code
                    having count(distinct {column}) = ?
                )",
                placeholders(params.len())
            ));
            let expected = params.len() as i64;
            query.push_values(params.into_iter().map(Value::Text));
            query.push_value(Value::Integer(expected));
        }
    }
    Ok(())
}

fn add_pendulum_clause(
    query: &mut StandardCardSqlQuery,
    left: Option<&NumericRangeFilterDto>,
    right: Option<&NumericRangeFilterDto>,
) {
    if left.is_none() && right.is_none() {
        return;
    }

    let mut clauses = vec!["p.code = r.code".to_string()];
    let mut params = Vec::new();
    if let Some(left) = left {
        if let Some(min) = left.min {
            clauses.push("p.left_scale >= ?".to_string());
            params.push(Value::Integer(min));
        }
        if let Some(max) = left.max {
            clauses.push("p.left_scale <= ?".to_string());
            params.push(Value::Integer(max));
        }
    }
    if let Some(right) = right {
        if let Some(min) = right.min {
            clauses.push("p.right_scale >= ?".to_string());
            params.push(Value::Integer(min));
        }
        if let Some(max) = right.max {
            clauses.push("p.right_scale <= ?".to_string());
            params.push(Value::Integer(max));
        }
    }
    if params.is_empty() {
        return;
    }
    query.push_clause(format!(
        "exists (
            select 1
            from standard_card_pendulum p
            where {}
        )",
        clauses.join(" and ")
    ));
    query.push_values(params);
}

fn add_setcode_clause(
    query: &mut StandardCardSqlQuery,
    filters: &NormalizedStandardCardSearchFilters,
) {
    if filters.setcodes.is_empty() {
        return;
    }
    let (column, values) = match filters.setcode_mode {
        SetcodeFilterModeDto::Exact => (
            "setcode",
            filters
                .setcodes
                .iter()
                .map(|value| *value as i64)
                .collect::<Vec<_>>(),
        ),
        SetcodeFilterModeDto::Base => (
            "base",
            filters
                .setcodes
                .iter()
                .map(|value| (*value & 0x0fff) as i64)
                .filter(|value| *value != 0)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>(),
        ),
    };
    if values.is_empty() {
        return;
    }
    match filters.setcode_match {
        CardFilterMatchModeDto::Any => {
            query.push_clause(format!(
                "exists (
                    select 1
                    from standard_card_setcodes sc
                    where sc.code = r.code
                      and sc.{column} in ({})
                )",
                placeholders(values.len())
            ));
            query.push_values(values.into_iter().map(Value::Integer));
        }
        CardFilterMatchModeDto::All => {
            query.push_clause(format!(
                "r.code in (
                    select code
                    from standard_card_setcodes
                    where {column} in ({})
                    group by code
                    having count(distinct {column}) = ?
                )",
                placeholders(values.len())
            ));
            let expected = values.len() as i64;
            query.push_values(values.into_iter().map(Value::Integer));
            query.push_value(Value::Integer(expected));
        }
    }
}

fn add_category_clause(
    query: &mut StandardCardSqlQuery,
    filters: &NormalizedStandardCardSearchFilters,
) -> AppResult<()> {
    let combined = filters
        .category_masks
        .iter()
        .fold(0u64, |acc, value| acc | *value);
    if combined == 0 {
        return Ok(());
    }
    let combined = u64_to_i64_param("category_mask", combined)?;
    match filters.category_match {
        CardFilterMatchModeDto::Any => {
            query.push_clause("(c.category & ?) != 0");
            query.push_value(Value::Integer(combined));
        }
        CardFilterMatchModeDto::All => {
            query.push_clause("(c.category & ?) = ?");
            query.push_value(Value::Integer(combined));
            query.push_value(Value::Integer(combined));
        }
    }
    Ok(())
}

fn load_card_list_page(
    connection: &Connection,
    language: &str,
    keyword: &str,
    filters: Option<&NormalizedStandardCardSearchFilters>,
    sort_by: &StandardCardSortFieldDto,
    sort_direction: &SortDirectionDto,
    page_size: u32,
    offset: usize,
) -> AppResult<(u64, Vec<CardListRowDto>)> {
    let order_by = card_order_by(sort_by, sort_direction);
    let query = build_card_sql_query(language, keyword, filters)?;
    let count_sql = query.count_sql();
    let total = connection
        .query_row(&count_sql, params_from_iter(query.params.iter()), |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|source| sqlite_query_error("standard_pack.sqlite_count_cards_failed", source))
        .and_then(|value| i64_to_u64("total", value))?;
    let mut page_params = query.params.clone();
    page_params.push(Value::Integer(page_size as i64));
    page_params.push(Value::Integer(usize_to_i64("offset", offset)?));
    let sql = query.page_sql(order_by);
    let rows = query_card_rows(connection, &sql, params_from_iter(page_params.iter()))?;
    Ok((total, rows))
}

fn query_card_rows<P>(
    connection: &Connection,
    sql: &str,
    params: P,
) -> AppResult<Vec<CardListRowDto>>
where
    P: rusqlite::Params,
{
    let mut statement = connection.prepare(sql).map_err(|source| {
        sqlite_query_error("standard_pack.sqlite_prepare_cards_failed", source)
    })?;
    let rows = statement
        .query_map(params, |row| {
            Ok(RawCardListRow {
                code: row.get(0)?,
                name: row.get(1)?,
                desc: row.get(2)?,
                primary_type: row.get(3)?,
                subtype_display: row.get(4)?,
                atk: row.get(5)?,
                def: row.get(6)?,
                level: row.get(7)?,
                has_image: row.get(8)?,
                has_script: row.get(9)?,
                has_field_image: row.get(10)?,
            })
        })
        .map_err(|source| sqlite_query_error("standard_pack.sqlite_query_cards_failed", source))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| sqlite_query_error("standard_pack.sqlite_read_cards_failed", source))?;

    rows.into_iter()
        .map(|row| {
            let code = i64_to_u32("code", row.code)?;
            Ok(CardListRowDto {
                id: code.to_string(),
                code,
                name: row.name,
                desc: row.desc,
                primary_type: deserialize_enum_text("primary_type", row.primary_type)?,
                subtype_display: row.subtype_display,
                atk: row.atk,
                def: row.def,
                level: row.level,
                has_image: sqlite_bool(row.has_image),
                has_script: sqlite_bool(row.has_script),
                has_field_image: sqlite_bool(row.has_field_image),
            })
        })
        .collect()
}

fn load_string_entries(connection: &Connection, language: &str) -> AppResult<Vec<PackStringEntry>> {
    let mut statement = connection
        .prepare(
            "select b.kind, b.key, coalesce(s.value, '')
             from standard_string_baseline b
             left join standard_strings s
               on s.kind = b.kind and s.key = b.key and s.language = ?1",
        )
        .map_err(|source| {
            sqlite_query_error("standard_pack.sqlite_prepare_strings_failed", source)
        })?;
    let rows = statement
        .query_map(params![language], |row| {
            Ok(RawStringEntry {
                kind: row.get(0)?,
                key: row.get(1)?,
                value: row.get(2)?,
            })
        })
        .map_err(|source| sqlite_query_error("standard_pack.sqlite_query_strings_failed", source))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| sqlite_query_error("standard_pack.sqlite_read_strings_failed", source))?;

    rows.into_iter()
        .map(|row| {
            Ok(PackStringEntry {
                kind: deserialize_enum_text("kind", row.kind)?,
                key: i64_to_u32("key", row.key)?,
                value: row.value,
            })
        })
        .collect()
}

fn load_setnames_from_sqlite(
    app_data_dir: &std::path::Path,
    language: &str,
) -> AppResult<Vec<StandardSetnameRecord>> {
    let connection = sqlite_store::open_readonly(app_data_dir)?;
    let mut statement = connection
        .prepare(
            "select key, value
             from standard_strings
             where kind = 'setname' and language = ?1",
        )
        .map_err(|source| {
            sqlite_query_error("standard_pack.sqlite_prepare_setnames_failed", source)
        })?;
    let mut rows = statement
        .query_map(params![language], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|source| sqlite_query_error("standard_pack.sqlite_query_setnames_failed", source))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| sqlite_query_error("standard_pack.sqlite_read_setnames_failed", source))?
        .into_iter()
        .map(|(key, value)| {
            Ok(StandardSetnameRecord {
                key: i64_to_u32("key", key)?,
                value,
            })
        })
        .collect::<AppResult<Vec<_>>>()?;
    rows.sort_by(|left, right| left.value.cmp(&right.value).then(left.key.cmp(&right.key)));
    Ok(rows)
}

fn load_namespace_baseline_from_sqlite(
    app_data_dir: &std::path::Path,
) -> AppResult<StandardNamespaceBaseline> {
    let connection = sqlite_store::open_readonly(app_data_dir)?;
    let mut baseline = StandardNamespaceBaseline::default();

    let mut code_statement = connection
        .prepare("select code from standard_code_baseline")
        .map_err(|source| {
            sqlite_query_error("standard_pack.sqlite_prepare_code_baseline_failed", source)
        })?;
    baseline.standard_codes = code_statement
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(|source| {
            sqlite_query_error("standard_pack.sqlite_query_code_baseline_failed", source)
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| {
            sqlite_query_error("standard_pack.sqlite_read_code_baseline_failed", source)
        })?
        .into_iter()
        .map(|code| i64_to_u32("code", code))
        .collect::<AppResult<BTreeSet<_>>>()?;
    drop(code_statement);

    let mut string_statement = connection
        .prepare("select kind, key from standard_string_baseline")
        .map_err(|source| {
            sqlite_query_error(
                "standard_pack.sqlite_prepare_string_baseline_failed",
                source,
            )
        })?;
    let string_rows = string_statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|source| {
            sqlite_query_error("standard_pack.sqlite_query_string_baseline_failed", source)
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| {
            sqlite_query_error("standard_pack.sqlite_read_string_baseline_failed", source)
        })?;
    for (kind, key) in string_rows {
        let kind: PackStringKind = deserialize_enum_text("kind", kind)?;
        let key = i64_to_u32("key", key)?;
        match kind {
            PackStringKind::System => {
                baseline.strings.system_keys.insert(key);
            }
            PackStringKind::Victory => {
                baseline.strings.victory_keys.insert(key);
            }
            PackStringKind::Counter => {
                baseline.strings.counter_keys.insert(key);
            }
            PackStringKind::Setname => {
                baseline.strings.setname_keys.insert(key);
            }
        }
    }
    drop(string_statement);

    let mut setname_base_statement = connection
        .prepare("select base from standard_setname_base_baseline")
        .map_err(|source| {
            sqlite_query_error(
                "standard_pack.sqlite_prepare_setname_base_baseline_failed",
                source,
            )
        })?;
    baseline.strings.setname_bases = setname_base_statement
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(|source| {
            sqlite_query_error(
                "standard_pack.sqlite_query_setname_base_baseline_failed",
                source,
            )
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| {
            sqlite_query_error(
                "standard_pack.sqlite_read_setname_base_baseline_failed",
                source,
            )
        })?
        .into_iter()
        .map(|base| i64_to_u16("base", base))
        .collect::<AppResult<BTreeSet<_>>>()?;

    Ok(baseline)
}

fn revision_from_manifest(manifest: &StandardPackSqliteManifest) -> u64 {
    manifest
        .indexed_at
        .timestamp_millis()
        .try_into()
        .unwrap_or_default()
}

fn format_string_key_hex(value: u32) -> String {
    format!("{value:X}")
}

fn card_order_by(
    sort_by: &StandardCardSortFieldDto,
    sort_direction: &SortDirectionDto,
) -> &'static str {
    match (sort_by, sort_direction) {
        (StandardCardSortFieldDto::Code, SortDirectionDto::Asc) => "r.code asc",
        (StandardCardSortFieldDto::Code, SortDirectionDto::Desc) => "r.code desc",
        (StandardCardSortFieldDto::Name, SortDirectionDto::Asc) => "r.name asc, r.code asc",
        (StandardCardSortFieldDto::Name, SortDirectionDto::Desc) => "r.name desc, r.code desc",
        (StandardCardSortFieldDto::Type, SortDirectionDto::Asc) => {
            "r.primary_type asc, r.subtype_display asc, r.code asc"
        }
        (StandardCardSortFieldDto::Type, SortDirectionDto::Desc) => {
            "r.primary_type desc, r.subtype_display desc, r.code desc"
        }
    }
}

fn build_fts_query(keyword: &str) -> String {
    keyword
        .split_whitespace()
        .filter(|part| !part.is_empty())
        .map(|part| format!("\"{}\"", part.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" OR ")
}

fn normalize_contains(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty())
}

fn normalize_range(value: Option<NumericRangeFilterDto>) -> Option<NumericRangeFilterDto> {
    value.filter(|range| range.min.is_some() || range.max.is_some())
}

fn unique_u32(values: Option<Vec<u32>>) -> Vec<u32> {
    values
        .unwrap_or_default()
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn unique_values<T: PartialEq>(values: Option<Vec<T>>) -> Vec<T> {
    let mut unique = Vec::new();
    for value in values.unwrap_or_default() {
        if !unique.contains(&value) {
            unique.push(value);
        }
    }
    unique
}

fn placeholders(count: usize) -> String {
    (0..count).map(|_| "?").collect::<Vec<_>>().join(", ")
}

fn enum_text_value<T: Serialize>(field: &str, value: &T) -> AppResult<String> {
    match serde_json::to_value(value).map_err(|source| {
        AppError::new(
            "standard_pack.sqlite_filter_serialize_failed",
            source.to_string(),
        )
        .with_detail("field", field)
    })? {
        serde_json::Value::String(value) => Ok(value),
        other => Err(AppError::new(
            "standard_pack.sqlite_filter_serialize_enum_failed",
            "serialized filter enum did not produce a string",
        )
        .with_detail("field", field)
        .with_detail("value", other)),
    }
}

fn deserialize_enum_text<T>(field: &str, value: String) -> AppResult<T>
where
    T: DeserializeOwned,
{
    serde_json::from_value(serde_json::Value::String(value.clone())).map_err(|source| {
        AppError::new(
            "standard_pack.sqlite_enum_deserialize_failed",
            source.to_string(),
        )
        .with_detail("field", field)
        .with_detail("value", value)
    })
}

fn sqlite_query_error(code: &str, source: rusqlite::Error) -> AppError {
    AppError::new(code, source.to_string())
}

fn sqlite_bool(value: i64) -> bool {
    value != 0
}

fn i64_to_u32(field: &str, value: i64) -> AppResult<u32> {
    u32::try_from(value).map_err(|_| {
        AppError::new(
            "standard_pack.sqlite_integer_out_of_range",
            "standard pack sqlite integer is out of range",
        )
        .with_detail("field", field)
        .with_detail("value", value)
    })
}

fn i64_to_u64(field: &str, value: i64) -> AppResult<u64> {
    u64::try_from(value).map_err(|_| {
        AppError::new(
            "standard_pack.sqlite_integer_out_of_range",
            "standard pack sqlite integer is out of range",
        )
        .with_detail("field", field)
        .with_detail("value", value)
    })
}

fn u64_to_i64_param(field: &str, value: u64) -> AppResult<i64> {
    i64::try_from(value).map_err(|_| {
        AppError::new(
            "standard_pack.sqlite_integer_out_of_range",
            "standard pack sqlite integer is out of range",
        )
        .with_detail("field", field)
        .with_detail("value", value)
    })
}

fn i64_to_u16(field: &str, value: i64) -> AppResult<u16> {
    u16::try_from(value).map_err(|_| {
        AppError::new(
            "standard_pack.sqlite_integer_out_of_range",
            "standard pack sqlite integer is out of range",
        )
        .with_detail("field", field)
        .with_detail("value", value)
    })
}

fn usize_to_i64(field: &str, value: usize) -> AppResult<i64> {
    i64::try_from(value).map_err(|_| {
        AppError::new(
            "standard_pack.sqlite_integer_out_of_range",
            "standard pack sqlite integer is out of range",
        )
        .with_detail("field", field)
        .with_detail("value", value)
    })
}

impl From<standard_pack::StandardPackStatus> for StandardPackStatusDto {
    fn from(value: standard_pack::StandardPackStatus) -> Self {
        let state = if !value.configured {
            StandardPackIndexStateDto::NotConfigured
        } else if !value.source_language_configured {
            StandardPackIndexStateDto::MissingLanguage
        } else if value.schema_mismatch {
            StandardPackIndexStateDto::MissingIndex
        } else if !value.index_exists {
            StandardPackIndexStateDto::MissingIndex
        } else if value.message.is_some() {
            StandardPackIndexStateDto::MissingSource
        } else if value.stale {
            StandardPackIndexStateDto::Stale
        } else {
            StandardPackIndexStateDto::Ready
        };

        Self {
            configured: value.configured,
            ygopro_path: value
                .ygopro_path
                .map(|path| path.to_string_lossy().to_string()),
            cdb_path: value
                .cdb_path
                .map(|path| path.to_string_lossy().to_string()),
            index_exists: value.index_exists,
            schema_mismatch: value.schema_mismatch,
            stale: value.stale,
            source_language: value.source_language,
            indexed_at: value.indexed_at,
            card_count: value.card_count,
            state,
            message: value.message,
        }
    }
}
