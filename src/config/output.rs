use serde::Deserialize;
use skia_safe::Color4f;

#[derive(Debug, Deserialize, Clone)]
pub struct OutputConfig {
    #[serde(deserialize_with = "crate::util::deserialize_hex_color")]
    #[serde(rename = "background-color")]
    pub background_color: Color4f,
}
