use std::path::{Path, PathBuf};

use crate::domain::resource::model::CardAssetState;

pub fn pack_pics_dir(pack_path: &Path) -> PathBuf {
    pack_path.join("pics")
}

pub fn pack_field_pics_dir(pack_path: &Path) -> PathBuf {
    pack_pics_dir(pack_path).join("field")
}

pub fn pack_scripts_dir(pack_path: &Path) -> PathBuf {
    pack_path.join("scripts")
}

pub fn card_image_path(pack_path: &Path, code: u32) -> PathBuf {
    pack_pics_dir(pack_path).join(format!("{code}.jpg"))
}

pub fn field_image_path(pack_path: &Path, code: u32) -> PathBuf {
    pack_field_pics_dir(pack_path).join(format!("{code}.jpg"))
}

pub fn script_path(pack_path: &Path, code: u32) -> PathBuf {
    pack_scripts_dir(pack_path).join(format!("c{code}.lua"))
}

pub fn detect_card_asset_state(pack_path: &Path, code: u32) -> CardAssetState {
    CardAssetState {
        has_image: card_image_path(pack_path, code).exists(),
        has_script: script_path(pack_path, code).exists(),
        has_field_image: field_image_path(pack_path, code).exists(),
    }
}

pub fn planned_asset_renames(pack_path: &Path, old_code: u32, new_code: u32) -> Vec<(PathBuf, PathBuf)> {
    [
        (card_image_path(pack_path, old_code), card_image_path(pack_path, new_code)),
        (field_image_path(pack_path, old_code), field_image_path(pack_path, new_code)),
        (script_path(pack_path, old_code), script_path(pack_path, new_code)),
    ]
    .into_iter()
    .collect()
}
