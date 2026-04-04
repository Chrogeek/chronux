use serde::Deserialize;
use skia_safe::Color4f;

#[derive(Deserialize, Debug, Clone)]
pub struct ExtraViewport {
    pub target: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ExtraViewportConfig {
    #[serde(default)]
    pub viewports: Vec<ExtraViewport>,
    #[serde(rename = "border-width")]
    pub border_width: f32,
    #[serde(rename = "border-color")]
    #[serde(deserialize_with = "crate::util::deserialize_hex_color")]
    pub border_color: Color4f,
}
