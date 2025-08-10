use std::collections::HashSet;

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct InputConfig {
    pub version: String,
    pub svg: String,
    #[serde(rename = "overlay")]
    pub overlay_svg: Option<String>,
    #[serde(default = "HashSet::new")]
    pub pinned: HashSet<String>,
}
