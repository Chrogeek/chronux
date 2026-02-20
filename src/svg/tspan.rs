use std::collections::HashMap;

use roxmltree::Node;
use skia_safe::{Canvas, Color, Matrix, Point, Rect, textlayout::TextDirection};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    animation::properties::Properties,
    config::counter::Counters,
    svg::{
        attribute_f32, attribute_str, attribute_transform, defs::SvgDef,
        placeholder_text::PlaceholderText,
    },
    util::{TextAnchor, TextDrawPass, error::ChronuxResult, paint::ColorPaint, trim_control_chars},
};

#[derive(Debug, Clone)]
pub struct Tspan {
    pub x: f32,
    pub y: f32,
    pub text: TspanText,
    pub style: Properties,
    pub length: f32,
    pub bounds: Rect,
}

#[derive(Debug, Clone)]
pub enum TspanText {
    Text(String),
    Placeholder(PlaceholderText),
}

impl Tspan {
    pub fn from_subtree(
        node: &Node,
        mat: Matrix,
        x: f32,
        y: f32,
        base_style: Properties,
        placeholder: &Option<PlaceholderText>,
    ) -> ChronuxResult<Vec<Tspan>> {
        fn dfs_node(
            node: &Node,
            mat: Matrix,
            x: f32,
            y: f32,
            base_style: Properties,
            result: &mut Vec<Tspan>,
            placeholder: &Option<PlaceholderText>,
        ) -> ChronuxResult<()> {
            let base_style = match attribute_str(node, "style") {
                Ok(style) => Properties::from_style(&style, mat, base_style)?,
                Err(_) => base_style,
            };

            for child in node.children() {
                let x = attribute_f32(&child, "x").unwrap_or(x);
                let y = attribute_f32(&child, "y").unwrap_or(y);
                if child.is_text() {
                    let text = child.text().unwrap_or("").to_string();
                    let text = trim_control_chars(&text);
                    let Point { x, y } = mat.map_point((x, y));
                    if !text.is_empty() {
                        let font = base_style.get_font()?;
                        let (width, bounds_rect) = font.measure_str(text, None);
                        let tspan = Tspan {
                            x,
                            y,
                            style: base_style.clone(),
                            length: width,
                            text: match placeholder {
                                Some(formatter) => TspanText::Placeholder(formatter.clone()),
                                None => TspanText::Text(text.to_string()),
                            },
                            bounds: bounds_rect,
                        };
                        result.push(tspan);
                    }
                    continue;
                } else {
                    let mat = mat * attribute_transform(&child);
                    dfs_node(&child, mat, x, y, base_style.clone(), result, placeholder)?;
                }
            }
            Ok(())
        }

        let mut result = Vec::new();

        let coeff = match base_style.text_anchor {
            TextAnchor::Start => 0.0,
            TextAnchor::Middle => 0.5,
            TextAnchor::End => 1.0,
        };

        dfs_node(node, mat, x, y, base_style, &mut result, placeholder)?;

        if result.is_empty() {
            return Ok(result);
        }

        let mut new_xy = Vec::new();
        let mut total_length = 0.0;
        let mut start_id = 0;
        let (mut sx, mut _sy) = (result[0].x, result[0].y);

        for (id, tsp) in result.iter().enumerate() {
            total_length += tsp.length;
            if id + 1 == result.len() || tsp.x != result[id + 1].x || tsp.y != result[id + 1].y {
                let mut cx = match tsp.style.dir {
                    TextDirection::LTR => sx - total_length * coeff,
                    TextDirection::RTL => sx + total_length * coeff,
                };
                for ret in result.iter().take(id + 1).skip(start_id) {
                    // start_id..=id
                    new_xy.push((cx + ret.length * coeff, ret.y));
                    cx += ret.length;
                }

                total_length = 0.0;
                start_id = id + 1;
                if id + 1 < result.len() {
                    sx = result[id + 1].x;
                    _sy = result[id + 1].y;
                }
            }
        }

        for (i, (x, y)) in new_xy.iter().enumerate() {
            result[i].x = *x;
            result[i].y = *y;
        }

        Ok(result)
    }

    pub fn draw_text(
        &self,
        canvas: &Canvas,
        tl: f32,
        tr: f32,
        properties: Properties,
        counters: &Counters,
        defs: &HashMap<String, SvgDef>,
        element_transform: Matrix,
        pass: TextDrawPass,
    ) -> ChronuxResult<()> {
        let tl = tl.max(0.0);
        let tr = tr.min(self.length);

        let (tx, ty) = (properties.translate_x, properties.translate_y);

        let color_style = if properties.tweening {
            properties
        } else {
            Properties::merge(&self.style, &properties)
        };

        let color_style = match pass {
            TextDrawPass::Stroke => Properties {
                fill_color: ColorPaint::Color(Color::TRANSPARENT),
                ..color_style
            },
            TextDrawPass::Fill => Properties {
                stroke_color: ColorPaint::Color(Color::TRANSPARENT),
                ..color_style
            },
        };

        let font = self.style.get_font()?;

        let paint = match pass {
            TextDrawPass::Stroke => color_style.get_stroke_paint(defs, element_transform),
            TextDrawPass::Fill => color_style.get_fill_paint(defs, element_transform),
        };

        let dir_coeff = match self.style.dir {
            TextDirection::LTR => 1.0,
            TextDirection::RTL => -1.0,
        };

        let offset = match self.style.text_anchor {
            TextAnchor::Start => 0.0,
            TextAnchor::Middle => self.length / 2.0,
            TextAnchor::End => self.length,
        };

        let text = match &self.text {
            TspanText::Text(text) => text.clone(),
            TspanText::Placeholder(placeholder) => placeholder.format(counters),
        };

        if tr - tl < 1e-4 {
            // Avoid drawing if the text length is too small
            // Do nothing here
        } else if tl < 1e-4 && tr > self.length - 1e-4 {
            let (x, y) = (self.x, self.y);
            canvas.draw_text_align(
                &text,
                (x + tx, y + ty),
                &font,
                &paint,
                self.style.text_anchor.to_align(self.style.dir),
            );
        } else {
            let mut dlen = 0.0;
            for g in text.graphemes(true) {
                let c = self.style.measure_string(g)?;

                if dlen + c < tl || dlen > tr {
                    // draw nothing
                } else {
                    let tl = tl.max(dlen);
                    let tr = tr.min(dlen + c);
                    let alpha = (tr - tl) / c;
                    let (x, y) = (self.x + (dlen - offset) * dir_coeff, self.y);

                    let mut new_paint = paint.clone();

                    new_paint.set_alpha((alpha * paint.alpha() as f32) as u8);
                    canvas.draw_text_align(
                        g,
                        (x + tx, y + ty),
                        &font,
                        &new_paint,
                        TextAnchor::Start.to_align(self.style.dir),
                    );
                }

                dlen += c;
            }
        }
        Ok(())
    }
}
