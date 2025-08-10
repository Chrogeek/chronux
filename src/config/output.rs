use serde::Deserialize;
use skia_safe::Color;

#[derive(Debug, Deserialize, Clone)]
pub struct OutputConfig {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    #[serde(rename = "time-start", default = "default_output_time")]
    pub time_start: f32,
    #[serde(rename = "time-end", default = "default_output_time")]
    pub time_end: f32,
    #[serde(deserialize_with = "crate::util::deserialize_hex_color")]
    #[serde(rename = "background-color")]
    pub background_color: Color,
}

fn default_output_time() -> f32 {
    f32::NAN
}
