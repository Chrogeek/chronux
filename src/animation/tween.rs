use serde::Deserialize;
use skia_safe::Color;

use crate::util::{parse_css_color, toml_value_as_number};

#[derive(Debug, Clone)]
pub struct Tween {
    pub opacity: Option<f32>,
    pub font_size: Option<f32>,
    pub font_weight: Option<f32>,
    pub fill_color: Option<Color>,
    pub stroke_color: Option<Color>,
    pub stroke_width: Option<f32>,
    pub translate_x: Option<f32>,
    pub translate_y: Option<f32>,
    // pub scale_x: Option<f32>,
    // pub scale_y: Option<f32>,
    pub rotate: Option<f32>,
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
                "stroke-width" => tween.stroke_width = toml_value_as_number(value).map(|v| v as f32),
                "opacity" => {
                    tween.opacity = toml_value_as_number(value).map(|v| v as f32);
                }
                "x" => tween.translate_x = toml_value_as_number(value).map(|v| v as f32),
                "y" => tween.translate_y = toml_value_as_number(value).map(|v| v as f32),
                "rotate" => tween.rotate = toml_value_as_number(value).map(|v| v as f32),
                _ => {}
            }
        }

        Ok(tween)
    }
}
