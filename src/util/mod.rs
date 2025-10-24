use std::{collections::HashMap, fmt::Display};

use once_cell::sync::OnceCell;
use serde::Deserialize;
use skia_safe::{Color, Paint};
use svgtypes::PaintOrder;

use crate::util::paint::ColorPaint;

pub mod fontmgr;
pub mod interpolator;
pub mod paint;
pub mod vector;
pub mod version;

#[derive(Debug)]
pub enum ChronuxError {
    VersionError(String),
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
    FfmpegError(String),
}

pub type ChronuxResult<T> = Result<T, ChronuxError>;

impl Display for ChronuxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChronuxError::VersionError(version) => {
                write!(f, "Incompatible version: {version}")
            }
            ChronuxError::IoError(e) => write!(f, "IO Error: {e}"),
            ChronuxError::ConfigError(e) => {
                write!(f, "Config Error at span '{:?}': {}", e.span(), e.message())
            }
            ChronuxError::XmlError(e) => write!(f, "SVG Parse Error: {e}"),
            ChronuxError::CircularDependency(id) => {
                write!(f, "Circular dependency detected: {id}")
            }
            ChronuxError::AttributeNotFound { node_id, attribute } => {
                if let Some(node_id) = node_id {
                    write!(f, "Attribute '{attribute}' not found in node '{node_id}'")
                } else {
                    write!(f, "Attribute '{attribute}' not found in node")
                }
            }
            ChronuxError::ParseFloatError(e) => write!(f, "Floating-point parse error: {e}"),
            ChronuxError::ColorParseError(color) => {
                write!(f, "Failed to parse color: '{color}'")
            }
            ChronuxError::SvgError(msg) => write!(f, "SVG Error: {msg}"),
            ChronuxError::SkiaError(msg) => write!(f, "Skia Error: {msg}"),
            ChronuxError::ArgError(msg) => write!(f, "Argument Error: {msg}"),
            ChronuxError::HrefError(msg) => write!(f, "Href Error: {msg}"),
            ChronuxError::BrokenDependency(id) => write!(f, "Broken dependency: {id}"),
            ChronuxError::FfmpegError(msg) => write!(f, "FFmpeg Error: {msg}"),
        }
    }
}

pub static COLOR_DICT: OnceCell<HashMap<String, Color>> = OnceCell::new();

pub fn parse_css_color(color: &str, opacity: f32) -> ChronuxResult<Color> {
    if color == "none" {
        return Ok(Color::TRANSPARENT);
    }
    if let Some(dict) = COLOR_DICT.get()
        && let Some(c) = dict.get(color)
    {
        return Ok(*c);
    }
    let [r, g, b, a] = csscolorparser::parse(color)
        .map_err(|e| {
            ChronuxError::ColorParseError(format!("cannot parse color literal '{color}': {e}"))
        })?
        .to_rgba8();
    let a = (opacity * a as f32).clamp(0.0, 255.0) as u8;
    Ok(Color::from_argb(a, r, g, b))
}

pub fn parse_color_or_image(color: &str, opacity: f32) -> ChronuxResult<ColorPaint> {
    if let Some(id) = parse_href(color) {
        return Ok(ColorPaint::Href(id));
    }
    parse_css_color(color, opacity).map(ColorPaint::Color)
}

pub fn parse_href(literal: &str) -> Option<String> {
    if literal.starts_with("url(#") && literal.ends_with(')') {
        Some(literal[5..literal.len() - 1].to_string())
    } else {
        None
    }
}

pub fn timecode(f: u32, fps: u32) -> String {
    let total_seconds = f / fps;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    let frames = f % fps;

    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}:{frames:02}")
    } else if minutes > 0 {
        format!("{minutes:02}:{seconds:02}:{frames:02}")
    } else {
        format!("{seconds:02}:{frames:02}")
    }
}

pub fn timecode_rt(t: f32, fps: u32) -> String {
    timecode((t * fps as f32) as u32, fps)
}

pub fn parse_unit_value(v: &str) -> ChronuxResult<f32> {
    if let Some(stripped) = v.strip_suffix("px") {
        stripped
            .parse::<f32>()
            .map_err(ChronuxError::ParseFloatError)
    } else if let Some(stripped) = v.strip_suffix("pt") {
        stripped
            .parse::<f32>()
            .map(|v| v * 3.0 / 4.0)
            .map_err(ChronuxError::ParseFloatError)
    } else {
        v.parse::<f32>()
            .map_err(|e| ChronuxError::ArgError(format!("Unsupported value '{v}': {e}")))
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

pub fn sort_paints(paint_order: PaintOrder, stroke: Paint, fill: Paint) -> [Paint; 2] {
    match paint_order.order {
        [
            svgtypes::PaintOrderKind::Fill,
            svgtypes::PaintOrderKind::Stroke,
            svgtypes::PaintOrderKind::Markers,
        ] => [fill, stroke],
        [
            svgtypes::PaintOrderKind::Fill,
            svgtypes::PaintOrderKind::Markers,
            svgtypes::PaintOrderKind::Stroke,
        ] => [fill, stroke],
        [
            svgtypes::PaintOrderKind::Stroke,
            svgtypes::PaintOrderKind::Fill,
            svgtypes::PaintOrderKind::Markers,
        ] => [stroke, fill],
        [
            svgtypes::PaintOrderKind::Stroke,
            svgtypes::PaintOrderKind::Markers,
            svgtypes::PaintOrderKind::Fill,
        ] => [stroke, fill],
        [
            svgtypes::PaintOrderKind::Markers,
            svgtypes::PaintOrderKind::Fill,
            svgtypes::PaintOrderKind::Stroke,
        ] => [fill, stroke],
        [
            svgtypes::PaintOrderKind::Markers,
            svgtypes::PaintOrderKind::Stroke,
            svgtypes::PaintOrderKind::Fill,
        ] => [stroke, fill],
        _ => unreachable!("Impossible paint order: {:?}", paint_order.order),
    }
}

pub fn trim_control_chars(s: &str) -> &str {
    let mut start = 0;
    let mut end = s.len();

    for (i, c) in s.char_indices() {
        if !c.is_control() {
            start = i;
            break;
        }
    }

    for (i, c) in s.char_indices().rev() {
        if !c.is_control() {
            end = i + c.len_utf8();
            break;
        }
    }

    &s[start..end]
}
