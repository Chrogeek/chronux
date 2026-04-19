use std::{collections::HashMap, str::FromStr};

use skia_safe::{
    Color4f, Font, Matrix, Paint, PaintCap, PaintJoin, PaintStyle, PathEffect, PathFillType,
    colors::TRANSPARENT, gradient_shader, image_filters::drop_shadow_only,
    textlayout::TextDirection,
};
use svgtypes::PaintOrder;

use crate::{
    animation::Animation,
    config::event::Event,
    svg::{defs::SvgDef, element::Shape},
    util::{
        self, TextAnchor,
        dict::RenderDict,
        error::{ChronuxError, ChronuxResult},
        paint::ColorPaint,
        parse_color_or_image, parse_font_weight, parse_href, parse_unit_value,
    },
};

#[derive(Debug, Clone)]
pub struct Properties {
    pub visible: bool,
    pub opacity: f32,
    pub font_family: String,
    pub font_size: f32,
    pub font_weight: f32,
    pub text_anchor: TextAnchor,
    pub dir: TextDirection,
    pub fill_color: ColorPaint,
    pub filter: Option<String>,
    pub stroke_color: ColorPaint,
    pub stroke_width: f32,
    pub stroke_linecap: PaintCap,
    pub stroke_linejoin: PaintJoin,
    pub paint_order: PaintOrder,
    pub translate_x: f32,
    pub translate_y: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub rotate: f32,
    pub trim: (f32, f32),
    pub typewriter: (f32, f32),
    pub sweep: (f32, f32),
    pub tweening: bool,
    pub dash_offset: f32,
    pub dash_array: Vec<f32>,
    pub shape: Shape,
    pub blur: Option<f32>,
    pub shadow: Option<f32>,
    pub fill_rule: PathFillType,
    pub small_fraction: f32,
}

impl Default for Properties {
    fn default() -> Self {
        Properties {
            visible: true,
            opacity: 1.0,
            font_family: "Arial".to_string(),
            font_size: 16.0,
            font_weight: 400.0,
            text_anchor: TextAnchor::Start, // default to left alignment
            dir: TextDirection::LTR,        // default to LTR
            fill_color: ColorPaint::default(), // transparent
            filter: None,
            stroke_color: ColorPaint::default(), // transparent
            stroke_width: 0.0,
            stroke_linecap: PaintCap::Round,   // default to butt cap
            stroke_linejoin: PaintJoin::Round, // default to miter join
            paint_order: Default::default(),
            translate_x: 0.0,
            translate_y: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            rotate: 0.0,
            trim: (0.0, 1.0),
            typewriter: (0.0, 1.0),
            sweep: (0.0, 1.0),
            tweening: false,
            dash_offset: 0.0,
            dash_array: Vec::new(),
            shape: Shape::None,
            blur: None,
            shadow: None,
            fill_rule: PathFillType::EvenOdd,
            small_fraction: 1.0,
        }
    }
}

impl Properties {
    pub fn apply(&self, event: &Event, t: f32, object: &str, dict: &RenderDict) -> Properties {
        match &event.animation {
            Animation::None => self.clone(),
            Animation::TrimPath { reverse } => {
                let chain = &event.object;
                if chain.is_atomic() {
                    let (start, end) = if *reverse { (1.0 - t, 1.0) } else { (0.0, t) };
                    Properties {
                        trim: (start, end),
                        ..self.clone()
                    }
                } else {
                    let segment = chain.locate(object, dict).unwrap_or((0.0, 0.0));
                    let seglength = segment.1 - segment.0;
                    let chain_len = chain.length(dict).unwrap_or(0.0);
                    let (start, end) = if *reverse { (1.0 - t, 1.0) } else { (0.0, t) };
                    let (start, end) = (start * chain_len, end * chain_len);
                    let (start, end) = (
                        start.clamp(segment.0, segment.1),
                        end.clamp(segment.0, segment.1),
                    );
                    let (start, end) = (
                        (start - segment.0) / seglength,
                        (end - segment.0) / seglength,
                    );
                    Properties {
                        trim: (start, end),
                        ..self.clone()
                    }
                }
            }
            Animation::TrimPathOut { reverse } => {
                let chain = &event.object;
                if chain.is_atomic() {
                    let (start, end) = if *reverse { (t, 1.0) } else { (0.0, 1.0 - t) };
                    Properties {
                        trim: (start, end),
                        ..self.clone()
                    }
                } else {
                    let segment = chain.locate(object, dict).unwrap_or((0.0, 0.0));
                    let seglength = segment.1 - segment.0;
                    let chain_len = chain.length(dict).unwrap_or(0.0);
                    let (start, end) = if *reverse { (t, 1.0) } else { (0.0, 1.0 - t) };
                    let (start, end) = (start * chain_len, end * chain_len);
                    let (start, end) = (
                        start.clamp(segment.0, segment.1),
                        end.clamp(segment.0, segment.1),
                    );
                    let (start, end) = (
                        (start - segment.0) / seglength,
                        (end - segment.0) / seglength,
                    );
                    Properties {
                        trim: (start, end),
                        ..self.clone()
                    }
                }
            }
            Animation::Typewriter { reverse } => Properties {
                typewriter: if *reverse { (1.0 - t, 1.0) } else { (0.0, t) },
                ..self.clone()
            },
            Animation::TypewriterOut { reverse } => Properties {
                typewriter: if *reverse { (0.0, 1.0 - t) } else { (t, 1.0) },
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
                if let Some(tx) = tween.translate_x {
                    new_props.translate_x += t * (tx - self.translate_x);
                }
                if let Some(ty) = tween.translate_y {
                    new_props.translate_y += t * (ty - self.translate_y);
                }
                if let Some(r) = tween.rotate {
                    new_props.rotate += t * (r - self.rotate);
                }
                if let Some(s) = &tween.shape {
                    // todo: handle shape tweening
                    let dst_shape = &dict.shapes[s];
                    let src_shape = &self.shape;
                    match (src_shape, dst_shape) {
                        (Shape::Path { d: src_d }, Shape::Path { d: dst_d }) => {
                            if let Some(res_d) = src_d.interpolate(dst_d, 1.0 - t) {
                                new_props.shape = Shape::Path { d: res_d };
                            } else {
                                log::error!(
                                    "Failed to interpolate between paths for shape tweening"
                                );
                                panic!();
                            }
                        }
                        _ => {
                            log::error!("Shape tweening requires two Path elements");
                            panic!();
                        }
                    }
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
            Animation::StretchIn => Properties {
                scale_x: t,
                scale_y: t,
                ..self.clone()
            },
            Animation::StretchOut => Properties {
                scale_x: 1.0 - t,
                scale_y: 1.0 - t,
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

        if let Some(rule) = rules.get("fill-rule") {
            result.fill_rule = match *rule {
                "nonzero" => PathFillType::Winding,
                "evenodd" => PathFillType::EvenOdd,
                _ => PathFillType::EvenOdd,
            };
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

        if let Some(filter) = rules.get("filter") {
            result.filter = parse_href(filter);
        }

        if let Some(width) = rules.get("stroke-width") {
            let w = parse_unit_value(width)?;
            result.stroke_width = mat.map_vector((w, 0.0)).length();
        }

        if let Some(size) = rules.get("font-size") {
            let s = parse_unit_value(size)?;
            result.font_size = mat.map_vector((s, 0.0)).length();
        }

        if let Some(weight) = rules.get("font-weight") {
            result.font_weight = parse_font_weight(weight)?;
        }

        if let Some(anchor) = rules.get("text-anchor") {
            result.text_anchor = match *anchor {
                "start" => TextAnchor::Start,
                "middle" => TextAnchor::Middle,
                "end" => TextAnchor::End,
                _ => TextAnchor::Start,
            };
        }

        if let Some(anchor) = rules.get("direction") {
            result.dir = match *anchor {
                "ltr" => TextDirection::LTR,
                "rtl" => TextDirection::RTL,
                _ => TextDirection::LTR,
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
            let o = parse_unit_value(dash_offset)?;
            result.dash_offset = mat.map_vector((o, 0.0)).length();
        }

        if let Some(dash_array) = rules.get("stroke-dasharray") {
            if dash_array.trim() == "none" {
                result.dash_array.clear();
            } else {
                let mut array = dash_array
                    .split(',')
                    .map(|s| parse_unit_value(s.trim()))
                    .collect::<ChronuxResult<Vec<_>>>()?
                    .into_iter()
                    .map(|v| mat.map_vector((v, 0.0)).length())
                    .collect::<Vec<_>>();
                if array.len() % 2 != 0 {
                    let dup = array.clone();
                    array.extend(&dup);
                }
                result.dash_array = array;
            }
        }

        if let Some(font_variations) = rules.get("font-variation-settings") {
            // Currently only support 'wght' axis
            for setting in font_variations.split(',') {
                let setting = setting.trim().split_ascii_whitespace().collect::<Vec<_>>();
                if setting.len() < 2 {
                    continue;
                }
                if setting[0] == "'wght'" || setting[0] == "\"wght\"" {
                    let weight = setting[1].trim().parse::<f32>().map_err(|_| {
                        ChronuxError::ArgError(format!(
                            "invalid font-variation-settings value: {font_variations}"
                        ))
                    })?;
                    result.font_weight = weight;
                }
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
                        let p1 = (element_transform * *gradient_transform).map_point(*p1);
                        let p2 = (element_transform * *gradient_transform).map_point(*p2);
                        let shader = gradient_shader::linear(
                            (p1, p2),
                            stops
                                .iter()
                                .map(|s| {
                                    let mut c = s.color;
                                    c.a *= self.opacity;
                                    c
                                })
                                .collect::<Vec<_>>()
                                .as_slice(),
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
                    // log::warn!(
                    //     "Linear gradient url(href) has incomplete properties: p1 = {p1:?}, p2 = {p2:?}"
                    // );
                    paint.set_color4f(TRANSPARENT, None);
                } else {
                    // log::warn!("Linear gradient url(href) not found: {href}");
                    paint.set_color4f(TRANSPARENT, None);
                }
            }
            ColorPaint::Color(c) => {
                let mut sc = c;
                sc.a *= self.opacity;
                paint.set_color4f(sc, None);
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
                        let p1 = (element_transform * *gradient_transform).map_point(*p1);
                        let p2 = (element_transform * *gradient_transform).map_point(*p2);
                        let shader = gradient_shader::linear(
                            (p1, p2),
                            stops
                                .iter()
                                .map(|s| {
                                    let mut c = s.color;
                                    c.a *= self.opacity;
                                    c
                                })
                                .collect::<Vec<_>>()
                                .as_slice(),
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
                    // log::warn!(
                    //     "Linear gradient url(href) has incomplete properties: p1 = {p1:?}, p2 = {p2:?}"
                    // );
                    paint.set_color4f(TRANSPARENT, None);
                } else {
                    // log::warn!("Linear gradient url(href) not found: {href}");
                    paint.set_color4f(TRANSPARENT, None);
                }
            }
            ColorPaint::Color(c) => {
                let mut fc = c;
                fc.a *= self.opacity;
                paint.set_color4f(fc, None);
            }
        }

        paint
    }

    pub fn get_shadow_paint(&self) -> Option<skia_safe::Paint> {
        let sh = self.shadow?;
        let shadow_color = Color4f::new(0.0, 0.0, 0.0, self.opacity);
        let shadow_filter = drop_shadow_only((0.0, 0.0), (sh, sh), shadow_color, None, None, None)?;
        let mut shadow_paint = Paint::default();
        shadow_paint
            .set_image_filter(shadow_filter)
            .set_style(PaintStyle::Stroke)
            .set_anti_alias(true);
        Some(shadow_paint)
    }

    pub fn get_effects(&self) -> Option<skia_safe::PathEffect> {
        let mut effects = vec![];

        // 260211补充说明：
        // dash必须放在trim前面，否则trim动画会变成一堆线段通过#seq连接起来的效果
        // 视觉上间断感很强，十分违和

        if self.dash_array.len() > 0 && self.stroke_width > 1e-4 {
            let dash_effect = skia_safe::dash_path_effect::new(&self.dash_array, self.dash_offset);
            effects.push(dash_effect);
        }

        if self.trim.0 > 0.0 || self.trim.1 < 1.0 {
            effects.push(skia_safe::trim_path_effect::new(
                self.trim.0,
                self.trim.1,
                skia_safe::trim_path_effect::Mode::Normal,
            ));
        }

        let effect = effects.into_iter().fold(None, |a, b| match (a, b) {
            (None, b) => b,
            (a, None) => a,
            (Some(a), Some(b)) => Some(PathEffect::compose(a, b)),
        });

        effect
    }

    pub fn get_font(&self) -> ChronuxResult<Font> {
        util::fontmgr::get_font(&self.font_family, self.font_size, self.font_weight as i32)
    }

    pub fn get_frac_font(&self) -> ChronuxResult<Font> {
        util::fontmgr::get_font(
            &self.font_family,
            self.font_size * self.small_fraction,
            (self.font_weight / self.small_fraction).round_ties_even() as i32,
        )
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
            scale_x: parent.scale_x * child.scale_x,
            scale_y: parent.scale_y * child.scale_y,
            rotate: parent.rotate + child.rotate,
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
            shape: match &child.shape {
                Shape::None => parent.shape.clone(),
                _ => child.shape.clone(),
            },
            ..child.clone()
        }
    }
}

fn tween_color(start: Color4f, end: Color4f, t: f32) -> Color4f {
    let a = start.a + t * (end.a - start.a);
    let r = start.r + t * (end.r - start.r);
    let g = start.g + t * (end.g - start.g);
    let b = start.b + t * (end.b - start.b);
    Color4f::new(r, g, b, a)
}

fn tween_color_or_image(start: ColorPaint, end: ColorPaint, t: f32) -> ColorPaint {
    match (start, end) {
        (ColorPaint::Color(s), ColorPaint::Color(e)) => ColorPaint::Color(tween_color(s, e, t)),
        // If either is an image reference, just return the end value
        (_, e) => e,
    }
}
