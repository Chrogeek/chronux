use roxmltree::Node;
use skia_safe::{Canvas, Matrix, Point, utils::text_utils::Align};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    animation::properties::Properties,
    svg::{attribute_f32, attribute_str, attribute_transform},
    util::{ChronuxResult, sort_paints},
};

#[derive(Debug, Clone)]
pub struct Tspan {
    pub x: f32,
    pub y: f32,
    pub text: String,
    pub style: Properties,
    pub length: f32,
}

impl Tspan {
    pub fn from_subtree(
        node: &Node,
        mat: Matrix,
        x: f32,
        y: f32,
        base_style: Properties,
    ) -> ChronuxResult<Vec<Tspan>> {
        fn dfs_node(
            node: &Node,
            mat: Matrix,
            x: f32,
            y: f32,
            base_style: Properties,
            result: &mut Vec<Tspan>,
        ) -> ChronuxResult<()> {
            let base_style = match attribute_str(node, "style") {
                Ok(style) => Properties::from_style(&style, mat, base_style)?,
                Err(_) => base_style,
            };

            for child in node.children() {
                let x = attribute_f32(&child, "x").unwrap_or(x);
                let y = attribute_f32(&child, "y").unwrap_or(y);
                if child.is_text() {
                    let text = child.text().unwrap_or("").trim().to_string();
                    let Point { x, y } = mat.map_point((x, y));
                    if !text.is_empty() {
                        let tspan = Tspan {
                            x,
                            y,
                            style: base_style.clone(),
                            length: base_style.measure_string(&text)?,
                            text,
                        };
                        result.push(tspan);
                    }
                    continue;
                } else {
                    let mat = mat * attribute_transform(&child);
                    dfs_node(&child, mat, x, y, base_style.clone(), result)?;
                }
            }
            Ok(())
        }

        let mut result = Vec::new();

        dfs_node(node, mat, x, y, base_style, &mut result)?;

        Ok(result)
    }

    pub fn draw_text(
        &self,
        canvas: &Canvas,
        tl: f32,
        tr: f32,
        properties: Properties,
    ) -> ChronuxResult<()> {
        let tl = tl.max(0.0);
        let tr = tr.min(self.length);

        let color_style = if properties.tweening {
            properties
        } else {
            self.style.clone()
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

        if tr - tl < 1e-3 {
            // Avoid drawing if the text length is too small
        } else if tl < 1e-3 && tr > self.length - 1e-3 {
            let (x, y) = (self.x, self.y);
            canvas.draw_text_align(
                &self.text,
                (x, y),
                &font,
                &paints[0],
                self.style.text_anchor,
            );
            canvas.draw_text_align(
                &self.text,
                (x, y),
                &font,
                &paints[1],
                self.style.text_anchor,
            );
        } else {
            let mut dlen = 0.0;
            for g in self.text.graphemes(true) {
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
                        canvas.draw_text_align(g, (x, y), &font, &paint, Align::Left);
                    }
                }

                dlen += c;
            }
        }
        Ok(())
    }
}
