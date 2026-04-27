use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::namespace::model::{
    PackStringNamespaceIndex, StandardNamespaceBaseline, StandardStringNamespaceBaseline,
};

pub fn load_standard_namespace_baseline() -> StandardNamespaceBaseline {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")));

    let standard_codes = load_standard_codes(repo_root.join("ref/ygopro_src/script"));
    let strings = load_standard_strings(repo_root.join("ref/expansions/ref/strings.conf"));

    StandardNamespaceBaseline {
        standard_codes,
        strings,
    }
}

fn load_standard_codes(script_dir: PathBuf) -> std::collections::BTreeSet<u32> {
    let mut codes = std::collections::BTreeSet::new();
    let Ok(entries) = fs::read_dir(script_dir) else {
        return codes;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        if !stem.starts_with('c') {
            continue;
        }
        let digits = &stem[1..];
        if digits.is_empty() || !digits.chars().all(|ch| ch.is_ascii_digit()) {
            continue;
        }
        if let Ok(code) = digits.parse::<u32>() {
            codes.insert(code);
        }
    }

    codes
}

fn load_standard_strings(path: PathBuf) -> StandardStringNamespaceBaseline {
    let Ok(contents) = fs::read_to_string(path) else {
        return StandardStringNamespaceBaseline::default();
    };

    let mut current_kind = None::<String>;
    let mut index = PackStringNamespaceIndex::default();

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(kind) = line.strip_prefix('!') {
            current_kind = Some(kind.to_ascii_lowercase());
            continue;
        }

        let Some(kind) = current_kind.as_deref() else {
            continue;
        };
        let mut parts = line.split_whitespace();
        let Some(raw_key) = parts.next() else {
            continue;
        };

        let parsed_key = match kind {
            "system" => raw_key.parse::<u32>().ok(),
            "victory" | "counter" | "setname" => u32::from_str_radix(raw_key, 16).ok(),
            _ => None,
        };
        let Some(key) = parsed_key else {
            continue;
        };

        let kind = match kind {
            "system" => crate::domain::strings::model::PackStringKind::System,
            "victory" => crate::domain::strings::model::PackStringKind::Victory,
            "counter" => crate::domain::strings::model::PackStringKind::Counter,
            "setname" => crate::domain::strings::model::PackStringKind::Setname,
            _ => continue,
        };
        index.insert_record(&crate::domain::strings::model::PackStringRecord {
            kind,
            key,
            values: std::collections::BTreeMap::new(),
        });
    }

    StandardStringNamespaceBaseline {
        system_keys: index.system_keys,
        victory_keys: index.victory_keys,
        counter_keys: index.counter_keys,
        setname_bases: index.setname_bases,
    }
}
