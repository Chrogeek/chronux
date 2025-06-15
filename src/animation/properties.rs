use skia_safe::{
    Color, Font, FontStyle, Matrix, Vector,
    font_style::{Slant, Width},
    utils::text_utils::Align,
};

use crate::{
    animation::Animation,
    util::{parse_css_color, parse_font_weight, parse_unit_value, ChronuxError, ChronuxResult},
};

#[derive(Debug, Clone)]
pub struct Properties {
    pub visible: bool,
    pub font_size: f32,
    pub font_weight: f32,
    pub text_anchor: Align,
    pub fill_color: Color,
    pub stroke_color: Color,
    pub stroke_width: f32,
    // pub translate_x: f32,
    // pub translate_y: f32,
    // pub scale_x: f32,
    // pub scale_y: f32,
    // pub rotate: f32,
    pub trim: (f32, f32),
    pub typewriter: (f32, f32),
    pub sweep: f32,
}

impl Default for Properties {
    fn default() -> Self {
        Properties {
            visible: true,
            font_size: 16.0,
            font_weight: 400.0,
            text_anchor: Align::Left,         // default to left alignment
            fill_color: Color::TRANSPARENT,   // transparent
            stroke_color: Color::TRANSPARENT, // transparent
            stroke_width: 0.0,
            // translate_x: 0.0,
            // translate_y: 0.0,
            // scale_x: 1.0,
            // scale_y: 1.0,
            // rotate: 0.0,
            trim: (0.0, 1.0),
            typewriter: (0.0, 1.0),
            sweep: 1.0,
        }
    }
}

impl Properties {
    pub fn apply(&self, event: &Animation, t: f32) -> Properties {
        match event {
            Animation::TrimPath { reverse } => {
                let (start, end) = if *reverse { (1.0 - t, 1.0) } else { (0.0, t) };
                Properties {
                    trim: (start, end),
                    ..self.clone()
                }
            }
            Animation::Typewriter => Properties {
                typewriter: (0.0, t),
                ..self.clone()
            },
            Animation::Sweep => Properties {
                sweep: t,
                ..self.clone()
            },
        }
    }

    pub fn from_style(style: &str, mat: Matrix) -> ChronuxResult<Properties> {
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
            let w = parse_unit_value(width)?;
            let Vector { x: w, y: _ } = mat.map_vector((w, 0.0));
            result.stroke_width = w;
        }

        if let Some(size) = rules.get("font-size") {
            let s = parse_unit_value(*size)?;
            let Vector { x: s, y: _ } = mat.map_vector((s, 0.0));
            result.font_size = s;
        }

        if let Some(weight) = rules.get("font-weight") {
            result.font_weight = parse_font_weight(*weight)?;
        }

        if let Some(anchor) = rules.get("text-anchor") {
            result.text_anchor = match *anchor {
                "start" => Align::Left,
                "middle" => Align::Center,
                "end" => Align::Right,
                _ => Align::Left, // default to left alignment
            };
        }

        Ok(result)
    }

    pub fn get_stroke_paint(&self) -> skia_safe::Paint {
        let mut paint = skia_safe::Paint::default();
        paint.set_anti_alias(true);
        paint.set_style(skia_safe::paint::Style::Stroke);
        paint.set_color(self.stroke_color);
        paint.set_stroke_width(self.stroke_width);
        paint
    }

    pub fn get_fill_paint(&self) -> skia_safe::Paint {
        let mut paint = skia_safe::Paint::default();
        paint.set_anti_alias(true);
        paint.set_style(skia_safe::paint::Style::Fill);
        paint.set_color(self.fill_color);
        paint
    }

    pub fn get_effects(&self) -> Vec<Option<skia_safe::PathEffect>> {
        let mut effects = Vec::new();

        effects.push(skia_safe::trim_path_effect::new(
            self.trim.0,
            self.trim.1,
            skia_safe::trim_path_effect::Mode::Normal, // reverse
        ));

        effects
    }

    pub fn get_font(&self) -> ChronuxResult<Font> {
        let mgr = skia_safe::FontMgr::default();
        let mut family = mgr.match_family("思源黑体");
        let tf = family
            .match_style(FontStyle::new(
                (self.font_weight as i32).into(),
                Width::NORMAL,
                Slant::Upright,
            ))
            .ok_or(ChronuxError::SkiaError(
                "failed to create typeface".to_string(),
            ))?;
        let font = Font::from_typeface(tf, self.font_size);
        Ok(font)
    }
}
