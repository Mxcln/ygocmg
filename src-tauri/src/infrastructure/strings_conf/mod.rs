use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::domain::common::error::{AppError, AppResult};
use crate::domain::namespace::model::{PackStringNamespaceIndex, StandardStringNamespaceBaseline};
use crate::domain::strings::model::{PackStringKind, PackStringRecord};

pub fn load_records(path: &Path) -> AppResult<Vec<PackStringRecord>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let contents = fs::read_to_string(path).map_err(|source| {
        AppError::from_io("strings_conf.read_failed", source)
            .with_detail("path", path.display().to_string())
    })?;
    Ok(parse_records(&contents))
}

pub fn parse_records(contents: &str) -> Vec<PackStringRecord> {
    let mut current_kind = None::<String>;
    let mut records = BTreeMap::<(PackStringKind, u32), PackStringRecord>::new();

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parsed = if let Some(rest) = line.strip_prefix('!') {
            let Some((kind, payload)) = split_first_token(rest.trim()) else {
                continue;
            };
            let kind = kind.to_ascii_lowercase();
            if payload.trim().is_empty() {
                current_kind = Some(kind);
                continue;
            }
            parse_record(&kind, payload)
        } else {
            let Some(kind) = current_kind.as_deref() else {
                continue;
            };
            parse_record(kind, line)
        };

        if let Some(record) = parsed {
            records.insert((record.kind.clone(), record.key), record);
        }
    }

    records.into_values().collect()
}

pub fn baseline_from_records(records: &[PackStringRecord]) -> StandardStringNamespaceBaseline {
    let mut index = PackStringNamespaceIndex::default();
    for record in records {
        index.insert_record(record);
    }
    StandardStringNamespaceBaseline {
        system_keys: index.system_keys,
        victory_keys: index.victory_keys,
        counter_keys: index.counter_keys,
        setname_keys: index.setname_keys,
        setname_bases: index.setname_bases,
    }
}

pub fn write_records(
    path: &Path,
    records: &[PackStringRecord],
    export_language: &str,
) -> AppResult<()> {
    let mut contents = String::new();
    for kind in [
        PackStringKind::System,
        PackStringKind::Victory,
        PackStringKind::Counter,
        PackStringKind::Setname,
    ] {
        let mut kind_records = records
            .iter()
            .filter(|record| record.kind == kind)
            .collect::<Vec<_>>();
        kind_records.sort_by_key(|record| record.key);
        if kind_records.is_empty() {
            continue;
        }

        contents.push('!');
        contents.push_str(kind_label(&kind));
        contents.push('\n');
        for record in kind_records {
            if let Some(value) = record.values.get(export_language) {
                contents.push_str(&format_record_key(&kind, record.key));
                contents.push(' ');
                contents.push_str(value.trim());
                contents.push('\n');
            }
        }
    }

    fs::write(path, contents).map_err(|source| {
        AppError::from_io("strings_conf.write_failed", source)
            .with_detail("path", path.display().to_string())
    })
}

fn parse_record(kind: &str, payload: &str) -> Option<PackStringRecord> {
    let (raw_key, value) = split_first_token(payload.trim())?;
    let kind = parse_kind(kind)?;
    let key = parse_key(&kind, raw_key)?;
    let mut values = BTreeMap::new();
    values.insert("default".to_string(), value.trim().to_string());
    Some(PackStringRecord { kind, key, values })
}

fn parse_kind(value: &str) -> Option<PackStringKind> {
    match value {
        "system" => Some(PackStringKind::System),
        "victory" => Some(PackStringKind::Victory),
        "counter" => Some(PackStringKind::Counter),
        "setname" => Some(PackStringKind::Setname),
        _ => None,
    }
}

fn kind_label(kind: &PackStringKind) -> &'static str {
    match kind {
        PackStringKind::System => "system",
        PackStringKind::Victory => "victory",
        PackStringKind::Counter => "counter",
        PackStringKind::Setname => "setname",
    }
}

fn format_record_key(kind: &PackStringKind, key: u32) -> String {
    if matches!(kind, PackStringKind::System) {
        key.to_string()
    } else {
        format!("0x{key:x}")
    }
}

fn parse_key(kind: &PackStringKind, raw: &str) -> Option<u32> {
    let raw = raw.trim();
    if matches!(kind, PackStringKind::System) && !raw.starts_with("0x") && !raw.starts_with("0X") {
        return raw.parse::<u32>().ok();
    }

    let hex = raw
        .strip_prefix("0x")
        .or_else(|| raw.strip_prefix("0X"))
        .unwrap_or(raw);
    u32::from_str_radix(hex, 16).ok()
}

fn split_first_token(value: &str) -> Option<(&str, &str)> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if let Some(index) = value.find(char::is_whitespace) {
        let (token, rest) = value.split_at(index);
        Some((token, rest.trim_start()))
    } else {
        Some((value, ""))
    }
}
