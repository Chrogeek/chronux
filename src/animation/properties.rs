use std::{collections::HashMap, str::FromStr};

use skia_safe::{
    Color, Font, Matrix, PaintCap, PaintJoin, Vector, gradient_shader, utils::text_utils::Align,
};
use svgtypes::PaintOrder;

use crate::{
    animation::Animation,
    svg::defs::SvgDef,
    util::{
        self, ChronuxError, ChronuxResult, paint::ColorPaint, parse_color_or_image,
        parse_font_weight, parse_unit_value,
    },
};

#[derive(Debug, Clone)]
pub struct Properties {
    pub visible: bool,
    pub opacity: f32,
    pub font_family: String,
    pub font_size: f32,
    pub font_weight: f32,
    pub text_anchor: Align,
    pub fill_color: ColorPaint,
    pub stroke_color: ColorPaint,
    pub stroke_width: f32,
    pub stroke_linecap: PaintCap,
    pub stroke_linejoin: PaintJoin,
    pub paint_order: PaintOrder,
    pub translate_x: f32,
    pub translate_y: f32,
    // pub scale_x: f32,
    // pub scale_y: f32,
    // pub rotate: f32,
    pub trim: (f32, f32),
    pub typewriter: (f32, f32),
    pub sweep: (f32, f32),
    pub tweening: bool,
    pub dash_offset: f32,
    pub dash_array: Vec<f32>,
}

impl Default for Properties {
    fn default() -> Self {
        Properties {
            visible: true,
            opacity: 1.0,
            font_family: "Arial".to_string(),
            font_size: 16.0,
            font_weight: 400.0,
            text_anchor: Align::Left,            // default to left alignment
            fill_color: ColorPaint::default(),   // transparent
            stroke_color: ColorPaint::default(), // transparent
            stroke_width: 0.0,
            stroke_linecap: PaintCap::Round,   // default to butt cap
            stroke_linejoin: PaintJoin::Round, // default to miter join
            paint_order: Default::default(),
            translate_x: 0.0,
            translate_y: 0.0,
            // scale_x: 1.0,
            // scale_y: 1.0,
            // rotate: 0.0,
            trim: (0.0, 1.0),
            typewriter: (0.0, 1.0),
            sweep: (0.0, 1.0),
            tweening: false,
            dash_offset: 0.0,
            dash_array: Vec::new(),
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
            Animation::TrimPathOut { reverse } => {
                let (start, end) = if *reverse { (t, 1.0) } else { (0.0, 1.0 - t) };
                Properties {
                    trim: (start, end),
                    ..self.clone()
                }
            }
            Animation::Typewriter => Properties {
                typewriter: (0.0, t),
                ..self.clone()
            },
            Animation::TypewriterOut => Properties {
                typewriter: (t, 1.0),
                ..self.clone()
            },
            Animation::Sweep => Properties {
                sweep: (0.0, t),
                ..self.clone()
            },
            Animation::SweepOut => Properties {
                sweep: (0.0, 1.0 - t),
                ..self.clone()
            },
            Animation::Tween(tween) => {
                let mut new_props = self.clone();
                if let Some(size) = tween.font_size {
                    new_props.font_size += t * (size - self.font_size);
                }
                if let Some(weight) = tween.font_weight {
                    new_props.font_weight += t * (weight - self.font_weight);
                }
                if let Some(color) = tween.fill_color {
                    new_props.fill_color =
                        tween_color_or_image(self.fill_color.clone(), ColorPaint::Color(color), t);
                }
                if let Some(color) = tween.stroke_color {
                    new_props.stroke_color = tween_color_or_image(
                        self.stroke_color.clone(),
                        ColorPaint::Color(color),
                        t,
                    );
                }
                if let Some(width) = tween.stroke_width {
                    new_props.stroke_width += t * (width - self.stroke_width);
                }
                if let Some(opacity) = tween.opacity {
                    new_props.opacity += t * (opacity - self.opacity);
                }
                new_props.tweening = true;
                new_props
            }
            Animation::FadeIn => Properties {
                opacity: t,
                ..self.clone()
            },
            Animation::FadeOut => Properties {
                opacity: 1.0 - t,
                ..self.clone()
            },
            Animation::Move { dx, dy } => Properties {
                translate_x: self.translate_x + t * dx,
                translate_y: self.translate_y + t * dy,
                ..self.clone()
            },
        }
    }

    pub fn from_style(
        style: &str,
        mat: Matrix,
        base_style: Properties,
    ) -> ChronuxResult<Properties> {
        let rules = style
            .split(';')
            .filter_map(|s| s.trim().split_once(':'))
            .map(|(k, v)| (k.trim(), v.trim()))
            .collect::<std::collections::HashMap<_, _>>();

        let mut result = base_style;

        if let Some(color) = rules.get("fill") {
            result.fill_color = parse_color_or_image(
                color,
                rules
                    .get("fill-opacity")
                    .and_then(|v| v.parse::<f32>().ok())
                    .unwrap_or(1.0),
            )?;
        }

        if let Some(color) = rules.get("stroke") {
            result.stroke_color = parse_color_or_image(
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
            let s = parse_unit_value(size)?;
            let Vector { x: s, y: _ } = mat.map_vector((s, 0.0));
            result.font_size = s;
        }

        if let Some(weight) = rules.get("font-weight") {
            result.font_weight = parse_font_weight(weight)?;
        }

        if let Some(anchor) = rules.get("text-anchor") {
            result.text_anchor = match *anchor {
                "start" => Align::Left,
                "middle" => Align::Center,
                "end" => Align::Right,
                _ => Align::Left,
            };
        }

        if let Some(cap) = rules.get("stroke-linecap") {
            result.stroke_linecap = match *cap {
                "butt" => PaintCap::Butt,
                "round" => PaintCap::Round,
                "square" => PaintCap::Square,
                _ => PaintCap::Round,
            };
        }

        if let Some(join) = rules.get("stroke-linejoin") {
            result.stroke_linejoin = match *join {
                "miter" => PaintJoin::Miter,
                "round" => PaintJoin::Round,
                "bevel" => PaintJoin::Bevel,
                _ => PaintJoin::Round,
            };
        }

        if let Some(font_family) = rules.get("font-family") {
            let ff = font_family.to_string();
            // dequote ff
            let ff = if (ff.starts_with('"') && ff.ends_with('"'))
                || (ff.starts_with('\'') && ff.ends_with('\''))
            {
                ff[1..ff.len() - 1].to_string()
            } else {
                ff
            };
            result.font_family = ff;
        }

        if let Some(paint_order) = rules.get("paint-order") {
            result.paint_order = PaintOrder::from_str(paint_order).map_err(|_| {
                ChronuxError::ArgError(format!("invalid paint-order value: {paint_order}"))
            })?;
        }

        if let Some(dash_offset) = rules.get("stroke-dashoffset") {
            result.dash_offset = parse_unit_value(dash_offset)?;
        }

        if let Some(dash_array) = rules.get("stroke-dasharray") {
            if dash_array.trim() == "none" {
                result.dash_array.clear();
            } else {
                let mut array = dash_array
                    .split(',')
                    .map(|s| parse_unit_value(s.trim()))
                    .collect::<ChronuxResult<Vec<_>>>()?;
                if array.len() % 2 != 0 {
                    let dup = array.clone();
                    array.extend(&dup);
                }
                result.dash_array = array;
            }
        }

        Ok(result)
    }

    pub fn get_stroke_paint(
        &self,
        defs: &HashMap<String, SvgDef>,
        element_transform: Matrix,
    ) -> skia_safe::Paint {
        let mut paint = skia_safe::Paint::default();
        paint.set_anti_alias(true);
        paint.set_style(skia_safe::paint::Style::Stroke);
        paint.set_stroke_width(self.stroke_width);
        paint.set_stroke_cap(self.stroke_linecap);
        paint.set_stroke_join(self.stroke_linejoin);

        match self.stroke_color {
            ColorPaint::Href(ref href) => {
                let def = defs.get(href);

                if let Some(SvgDef::LinearGradient {
                    p1,
                    p2,
                    stops,
                    gradient_transform,
                    ..
                }) = def
                {
                    if let (Some(p1), Some(p2)) = (p1, p2) {
                        let p1 = (*gradient_transform * element_transform).map_point(*p1);
                        let p2 = (*gradient_transform * element_transform).map_point(*p2);
                        let shader = gradient_shader::linear(
                            (p1, p2),
                            stops.iter().map(|s| s.color).collect::<Vec<_>>().as_slice(),
                            stops
                                .iter()
                                .map(|s| s.offset)
                                .collect::<Vec<_>>()
                                .as_slice(),
                            skia_safe::TileMode::Clamp,
                            None,
                            None,
                        );
                        paint.set_shader(shader);
                        return paint;
                    }
                    log::warn!(
                        "Linear gradient url(href) has incomplete properties: p1 = {p1:?}, p2 = {p2:?}"
                    );
                    paint.set_color(Color::TRANSPARENT);
                } else {
                    log::warn!("Linear gradient url(href) not found: {href}");
                    paint.set_color(Color::TRANSPARENT);
                }
            }
            ColorPaint::Color(c) => {
                let sc = c.with_a((c.a() as f32 * self.opacity) as u8);
                paint.set_color(sc);
            }
        }

        paint
    }

    pub fn get_fill_paint(
        &self,
        defs: &HashMap<String, SvgDef>,
        element_transform: Matrix,
    ) -> skia_safe::Paint {
        let mut paint = skia_safe::Paint::default();
        paint.set_anti_alias(true);
        paint.set_style(skia_safe::paint::Style::Fill);

        match self.fill_color {
            ColorPaint::Href(ref href) => {
                let def = defs.get(href);

                if let Some(SvgDef::LinearGradient {
                    p1,
                    p2,
                    stops,
                    gradient_transform,
                    ..
                }) = def
                {
                    if let (Some(p1), Some(p2)) = (p1, p2) {
                        let p1 = (*gradient_transform * element_transform).map_point(*p1);
                        let p2 = (*gradient_transform * element_transform).map_point(*p2);
                        let shader = gradient_shader::linear(
                            (p1, p2),
                            stops.iter().map(|s| s.color).collect::<Vec<_>>().as_slice(),
                            stops
                                .iter()
                                .map(|s| s.offset)
                                .collect::<Vec<_>>()
                                .as_slice(),
                            skia_safe::TileMode::Clamp,
                            None,
                            None,
                        );
                        paint.set_shader(shader);
                        return paint;
                    }
                    log::warn!(
                        "Linear gradient url(href) has incomplete properties: p1 = {p1:?}, p2 = {p2:?}"
                    );
                    paint.set_color(Color::TRANSPARENT);
                } else {
                    log::warn!("Linear gradient url(href) not found: {href}");
                    paint.set_color(Color::TRANSPARENT);
                }
            }
            ColorPaint::Color(c) => {
                let fc = c.with_a((c.a() as f32 * self.opacity) as u8);
                paint.set_color(fc);
            }
        }

        paint
    }

    pub fn get_effects(&self) -> Vec<Option<skia_safe::PathEffect>> {
        vec![skia_safe::trim_path_effect::new(
            self.trim.0,
            self.trim.1,
            skia_safe::trim_path_effect::Mode::Normal, // reverse
        )]
    }

    pub fn get_font(&self) -> ChronuxResult<Font> {
        util::fontmgr::get_font(&self.font_family, self.font_size, self.font_weight as i32)
    }

    pub fn measure_string(&self, text: &str) -> ChronuxResult<f32> {
        let font = self.get_font()?;
        let (width, _rect) = font.measure_str(text, None);
        Ok(width)
    }

    pub fn merge(child: &Properties, parent: &Properties) -> Properties {
        Properties {
            visible: child.visible && parent.visible,
            opacity: child.opacity * parent.opacity,
            font_family: if child.font_family.is_empty() {
                parent.font_family.clone()
            } else {
                child.font_family.clone()
            },
            translate_x: parent.translate_x + child.translate_x,
            translate_y: parent.translate_y + child.translate_y,
            // scale_x: parent.scale_x * child.scale_x,
            // scale_y: parent.scale_y * child.scale_y,
            // rotate: parent.rotate + child.rotate,
            trim: (
                parent.trim.0.max(child.trim.0),
                parent.trim.1.min(child.trim.1),
            ),
            typewriter: (
                parent.typewriter.0.max(child.typewriter.0),
                parent.typewriter.1.min(child.typewriter.1),
            ),
            sweep: (
                parent.sweep.0.max(child.sweep.0),
                parent.sweep.1.min(child.sweep.1),
            ),
            tweening: false, // TODO handle tweening properly
            dash_offset: if !child.dash_array.is_empty() {
                child.dash_offset
            } else {
                parent.dash_offset
            },
            dash_array: if !child.dash_array.is_empty() {
                child.dash_array.clone()
            } else {
                parent.dash_array.clone()
            },
            ..child.clone()
        }
    }
}

fn tween_color(start: Color, end: Color, t: f32) -> Color {
    let a = (start.a() as f32 + t * (end.a() as f32 - start.a() as f32)) as u8;
    let r = (start.r() as f32 + t * (end.r() as f32 - start.r() as f32)) as u8;
    let g = (start.g() as f32 + t * (end.g() as f32 - start.g() as f32)) as u8;
    let b = (start.b() as f32 + t * (end.b() as f32 - start.b() as f32)) as u8;
    Color::from_argb(a, r, g, b)
}

fn tween_color_or_image(start: ColorPaint, end: ColorPaint, t: f32) -> ColorPaint {
    match (start, end) {
        (ColorPaint::Color(s), ColorPaint::Color(e)) => ColorPaint::Color(tween_color(s, e, t)),
        // If either is an image reference, just return the end value
        (_, e) => e,
    }
}
