use std::collections::HashSet;

use serde::Deserialize;
use serde_with::serde_as;

#[serde_as]
#[derive(Debug, Deserialize, Clone)]
pub struct ExtraStrokeConfig {
    #[serde_with(as = "OneOrMany<_, PreferOne>")]
    pub paths: HashSet<String>,
    // #[serde(rename = "path-groups")]
    // pub path_groups: Vec<Vec<String>>,
    #[serde(rename = "stroke-color")]
    pub stroke_color: String,
    #[serde(rename = "stroke-width")]
    pub stroke_width: f32,
}
