use skia_safe::Color;

use crate::{
    animation::Animation,
    util::{ChronuxError, ChronuxResult, parse_css_color},
};

#[derive(Debug, Clone)]
pub struct Properties {
    pub visible: bool,
    pub font_size: f64,
    pub font_weight: f64,
    pub fill_color: Color,
    pub stroke_color: Color,
    pub stroke_width: f64,
    pub translate_x: f64,
    pub translate_y: f64,
    pub scale_x: f64,
    pub scale_y: f64,
    // pub rotate: f64,
    pub trim: (f64, f64),
}

impl Default for Properties {
    fn default() -> Self {
        Properties {
            visible: true,
            font_size: 16.0,
            font_weight: 400.0,
            fill_color: Color::TRANSPARENT,   // transparent
            stroke_color: Color::TRANSPARENT, // transparent
            stroke_width: 1.0,
            translate_x: 0.0,
            translate_y: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            // rotate: 0.0,
            trim: (0.0, 1.0),
        }
    }
}

impl Properties {
    pub fn apply(&self, event: &Animation, t: f64) -> Properties {
        match event {
            Animation::TrimPath { reverse } => {
                let (start, end) = if *reverse { (1.0 - t, 1.0) } else { (0.0, t) };
                Properties {
                    trim: (start, end),
                    ..self.clone()
                }
            } // Add other animation types here as needed
        }
    }

    pub fn from_style(style: &str) -> ChronuxResult<Properties> {
        let rules = style
            .split(';')
            .filter_map(|s| s.trim().split_once(':'))
            .map(|(k, v)| (k.trim(), v.trim()))
            .collect::<std::collections::HashMap<_, _>>();

        let mut result = Properties::default();

        if let Some(color) = rules.get("fill") {
            result.fill_color = parse_css_color(
                color,
                rules
                    .get("fill-opacity")
                    .and_then(|v| v.parse::<f32>().ok())
                    .unwrap_or(1.0),
            )?;
        }

        if let Some(color) = rules.get("stroke") {
            result.stroke_color = parse_css_color(
                color,
                rules
                    .get("stroke-opacity")
                    .and_then(|v| v.parse::<f32>().ok())
                    .unwrap_or(1.0),
            )?;
        }

        if let Some(width) = rules.get("stroke-width") {
            result.stroke_width = width
                .parse::<f64>()
                .map_err(ChronuxError::ParseFloatError)?;
        }

        if let Some(size) = rules.get("font-size") {
            result.font_size = size
                .replace("px", "")
                .parse::<f64>()
                .map_err(ChronuxError::ParseFloatError)?;
        }

        if let Some(weight) = rules.get("font-weight") {
            result.font_weight = weight
                .parse::<f64>()
                .map_err(ChronuxError::ParseFloatError)?;
        }

        Ok(result)
    }
}
