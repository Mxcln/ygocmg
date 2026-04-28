use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::namespace::model::{StandardNamespaceBaseline, StandardStringNamespaceBaseline};

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
    crate::infrastructure::strings_conf::load_records(&path)
        .map(|records| crate::infrastructure::strings_conf::baseline_from_records(&records))
        .unwrap_or_default()
}
