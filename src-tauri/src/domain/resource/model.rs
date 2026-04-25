use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CardAssetState {
    pub has_image: bool,
    pub has_script: bool,
    pub has_field_image: bool,
}
