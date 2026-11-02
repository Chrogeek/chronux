use std::fmt::Display;

use serde::Deserialize;
use skia_safe::Color4f;

use crate::util::{parse_css_color, toml_value_as_number};

#[derive(Debug, Clone)]
pub struct Tween {
    pub opacity: Option<f32>,
    pub font_size: Option<f32>,
    pub font_weight: Option<f32>,
    pub fill_color: Option<Color4f>,
    pub stroke_color: Option<Color4f>,
    pub stroke_width: Option<f32>,
    pub translate_x: Option<f32>,
    pub translate_y: Option<f32>,
    // pub scale_x: Option<f32>,
    // pub scale_y: Option<f32>,
    pub rotate: Option<f32>,
    pub shape: Option<String>,
}

impl<'de> Deserialize<'de> for Tween {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut tween = Tween {
            opacity: None,
            font_size: None,
            font_weight: None,
            fill_color: None,
            stroke_color: None,
            stroke_width: None,
            translate_x: None,
            translate_y: None,
            rotate: None,
            shape: None,
        };

        let map = toml::Table::deserialize(deserializer)?;
        for (key, value) in map.iter() {
            match key.as_str() {
                "font-size" => tween.font_size = toml_value_as_number(value).map(|v| v as f32),
                "font-weight" => tween.font_weight = toml_value_as_number(value).map(|v| v as f32),
                "fill-color" => {
                    tween.fill_color = {
                        if let Some(c) = value.as_str().map(|c| parse_css_color(c, 1.0)) {
                            Some(c.map_err(|e| serde::de::Error::custom(e.to_string()))?)
                        } else {
                            None
                        }
                    }
                }
                "stroke-color" => {
                    tween.stroke_color = {
                        if let Some(c) = value.as_str().map(|c| parse_css_color(c, 1.0)) {
                            Some(c.map_err(|e| serde::de::Error::custom(e.to_string()))?)
                        } else {
                            None
                        }
                    }
                }
                "stroke-width" => {
                    tween.stroke_width = toml_value_as_number(value).map(|v| v as f32)
                }
                "opacity" => {
                    tween.opacity = toml_value_as_number(value).map(|v| v as f32);
                }
                "x" => tween.translate_x = toml_value_as_number(value).map(|v| v as f32),
                "y" => tween.translate_y = toml_value_as_number(value).map(|v| v as f32),
                "rotate" => tween.rotate = toml_value_as_number(value).map(|v| v as f32),
                "shape" => tween.shape = value.as_str().map(|s| s.to_string()),
                _ => {}
            }
        }

        Ok(tween)
    }
}

impl Display for Tween {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut output_lists = Vec::new();
        if let Some(o) = self.opacity {
            output_lists.push(format!("opacity = {}", o));
        }
        if let Some(fs) = self.font_size {
            output_lists.push(format!("font-size = {}", fs));
        }
        if let Some(fw) = self.font_weight {
            output_lists.push(format!("font-weight = {}", fw));
        }
        if let Some(fc) = self.fill_color {
            output_lists.push(format!("fill-color = {:?}", fc));
        }
        if let Some(sc) = self.stroke_color {
            output_lists.push(format!("stroke-color = {:?}", sc));
        }
        if let Some(sw) = self.stroke_width {
            output_lists.push(format!("stroke-width = {}", sw));
        }
        if let Some(tx) = self.translate_x {
            output_lists.push(format!("translate-x = {}", tx));
        }
        if let Some(ty) = self.translate_y {
            output_lists.push(format!("translate-y = {}", ty));
        }
        if let Some(r) = self.rotate {
            output_lists.push(format!("rotate = {}", r));
        }
        if let Some(s) = &self.shape {
            output_lists.push(format!("shape = {}", s));
        }
        if output_lists.len() == 0 {
            write!(f, "Tween(empty)")
        } else {
            write!(f, "Tween({})", output_lists.join(", "))
        }
    }
}
