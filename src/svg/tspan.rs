use roxmltree::Node;
use skia_safe::{Canvas, Matrix, Point, Rect, utils::text_utils::Align};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    animation::properties::Properties,
    config::counter::Counters,
    svg::{attribute_f32, attribute_str, attribute_transform, placeholder_text::PlaceholderText},
    util::{ChronuxResult, sort_paints},
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
                    let Point { x, y } = mat.map_point((x, y));
                    if !text.is_empty() {
                        let font = base_style.get_font()?;
                        let (width, bounds_rect) = font.measure_str(&text, None);
                        let tspan = Tspan {
                            x,
                            y,
                            style: base_style.clone(),
                            length: width,
                            text: match placeholder {
                                Some(formatter) => TspanText::Placeholder(formatter.clone()),
                                None => TspanText::Text(text),
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
            Align::Left => 0.0,
            Align::Center => 0.5,
            Align::Right => 1.0,
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
                let mut cx = sx - total_length * coeff;
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
    ) -> ChronuxResult<()> {
        let tl = tl.max(0.0);
        let tr = tr.min(self.length);

        let (tx, ty) = (properties.translate_x, properties.translate_y);

        let color_style = if properties.tweening {
            properties
        } else {
            Properties::merge(&self.style, &properties)
        };

        let font = self.style.get_font()?;
        let fill = color_style.get_fill_paint();
        let stroke = color_style.get_stroke_paint();

        let paints = sort_paints(color_style.paint_order, stroke, fill);

        let offset = match self.style.text_anchor {
            Align::Left => 0.0,
            Align::Center => self.length / 2.0,
            Align::Right => self.length,
        };

        let text = match &self.text {
            TspanText::Text(text) => text.clone(),
            TspanText::Placeholder(placeholder) => placeholder.format(counters),
        };

        if tr - tl < 1e-3 {
            // Avoid drawing if the text length is too small
            // Do nothing here
        } else if tl < 1e-3 && tr > self.length - 1e-3 {
            let (x, y) = (self.x, self.y);
            canvas.draw_text_align(
                &text,
                (x + tx, y + ty),
                &font,
                &paints[0],
                self.style.text_anchor,
            );
            canvas.draw_text_align(
                &text,
                (x + tx, y + ty),
                &font,
                &paints[1],
                self.style.text_anchor,
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
                    let (x, y) = (self.x + dlen - offset, self.y);

                    let mut new_paints = paints.clone();

                    for paint in new_paints.iter_mut() {
                        // paint.set_path_effect(self.style.get_path_effect());
                        paint.set_alpha((alpha * paint.alpha() as f32) as u8);
                        canvas.draw_text_align(g, (x + tx, y + ty), &font, paint, Align::Left);
                    }
                }

                dlen += c;
            }
        }
        Ok(())
    }
}
