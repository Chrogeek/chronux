use std::fmt::Display;

use serde::Deserialize;
use skia_safe::Color;

pub mod vector;

#[derive(Debug)]
pub enum ChronuxError {
    IoError(std::io::Error),
    ConfigError(toml::de::Error),
    XmlError(roxmltree::Error),
    CircularDependency(String),
    AttributeNotFound {
        node_id: Option<String>,
        attribute: String,
    },
    ParseFloatError(std::num::ParseFloatError),
    ColorParseError(String),
    SvgError(String),
    SkiaError(String),
    ArgError(String),
    HrefError(String),
    BrokenDependency(String),
}

pub type ChronuxResult<T> = Result<T, ChronuxError>;

impl Display for ChronuxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChronuxError::IoError(e) => write!(f, "IO Error: {}", e),
            ChronuxError::ConfigError(e) => write!(f, "Config Error at span '{:?}': {}", e.span(), e.message()),
            ChronuxError::XmlError(e) => write!(f, "SVG Parse Error: {}", e),
            ChronuxError::CircularDependency(id) => {
                write!(f, "Circular dependency detected: {}", id)
            }
            ChronuxError::AttributeNotFound { node_id, attribute } => {
                if let Some(node_id) = node_id {
                    write!(
                        f,
                        "Attribute '{}' not found in node '{}'",
                        attribute, node_id
                    )
                } else {
                    write!(f, "Attribute '{}' not found in node", attribute)
                }
            }
            ChronuxError::ParseFloatError(e) => write!(f, "Floating-point parse error: {}", e),
            ChronuxError::ColorParseError(color) => {
                write!(f, "Failed to parse color: '{}'", color)
            }
            ChronuxError::SvgError(msg) => write!(f, "SVG Error: {}", msg),
            ChronuxError::SkiaError(msg) => write!(f, "Skia Error: {}", msg),
            ChronuxError::ArgError(msg) => write!(f, "Argument Error: {}", msg),
            ChronuxError::HrefError(msg) => write!(f, "Href Error: {}", msg),
            ChronuxError::BrokenDependency(id) => write!(f, "Broken dependency: {}", id),
        }
    }
}

pub fn parse_css_color(color: &str, opacity: f32) -> ChronuxResult<Color> {
    if color == "none" {
        return Ok(Color::TRANSPARENT);
    }
    let [r, g, b, _] = csscolorparser::parse(color)
        .map_err(|e| {
            ChronuxError::ColorParseError(format!(
                "cannot parse color literal '{}': {}",
                color.to_string(),
                e.to_string()
            ))
        })?
        .to_rgba8();
    let a = (opacity * 255.0).clamp(0.0, 255.0) as u8;
    Ok(Color::from_argb(a, r, g, b))
}

// pub fn timecode(f: u32, fps: u32) -> String {
//     let total_seconds = f / fps;
//     let hours = total_seconds / 3600;
//     let minutes = (total_seconds % 3600) / 60;
//     let seconds = total_seconds % 60;
//     let frames = f % fps;

//     if hours > 0 {
//         format!("{:02}:{:02}:{:02}:{:02}", hours, minutes, seconds, frames)
//     } else if minutes > 0 {
//         format!("{:02}:{:02}:{:02}", minutes, seconds, frames)
//     } else {
//         format!("{:02}:{:02}", seconds, frames)
//     }
// }

pub fn parse_unit_value(v: &str) -> ChronuxResult<f32> {
    if v.ends_with("px") {
        v[..v.len() - 2]
            .parse::<f32>()
            .map_err(ChronuxError::ParseFloatError)
    } else if v.ends_with("pt") {
        v[..v.len() - 2]
            .parse::<f32>()
            .map(|v| v * 3.0 / 4.0)
            .map_err(ChronuxError::ParseFloatError)
    } else {
        v.parse::<f32>()
            .map_err(|e| ChronuxError::ArgError(format!("Unsupported value '{}': {}", v, e)))
    }
}

pub fn parse_font_weight(weight: &str) -> ChronuxResult<f32> {
    match weight {
        "thin" => Ok(100.0),
        "hairline" => Ok(100.0),
        "extralight" => Ok(200.0),
        "ultralight" => Ok(200.0),
        "light" => Ok(300.0),
        "lighter" => Ok(300.0),
        "regular" => Ok(400.0),
        "normal" => Ok(400.0),
        "medium" => Ok(500.0),
        "semibold" => Ok(600.0),
        "demibold" => Ok(600.0),
        "bold" => Ok(700.0),
        "bolder" => Ok(800.0),
        "extrabold" => Ok(800.0),
        "ultrabold" => Ok(900.0),
        "black" => Ok(900.0),
        "heavy" => Ok(900.0),
        _ => weight.parse::<f32>().map_err(ChronuxError::ParseFloatError),
    }
}

pub fn deserialize_hex_color<'de, D>(deserializer: D) -> Result<Color, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let hex_str: String = Deserialize::deserialize(deserializer)?;
    parse_css_color(&hex_str, 1.0).map_err(serde::de::Error::custom)
}
