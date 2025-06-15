use roxmltree::Node;
use skia_safe::{Matrix, Path, Point, Vector};

use crate::{animation::properties::Properties, svg, util::ChronuxResult};

#[derive(Debug)]
pub struct Object {
    pub id: String,
    pub style: Properties,
    pub shape: Shape,
}

#[derive(Debug)]
pub enum Shape {
    Path { d: Path },
    Circle { cx: f32, cy: f32, r: f32 },
    Text { x: f32, y: f32, content: String },
}

impl Object {
    pub fn from_node(node: &Node, id: String, mat: Matrix) -> ChronuxResult<Object> {
        let style = Properties::from_style(&svg::attribute_str(node, "style")?, mat)?;
        Ok(Object {
            id,
            style,
            shape: match node.tag_name().name() {
                "path" => Shape::Path {
                    d: svg::path::svg_to_path(&svg::attribute_str(node, "d")?, mat)?,
                },
                "circle" => {
                    let (cx, cy, r) = (
                        svg::attribute_f32(node, "cx")?,
                        svg::attribute_f32(node, "cy")?,
                        svg::attribute_f32(node, "r")?,
                    );
                    let Point { x: cx, y: cy } = mat.map_point((cx, cy));
                    let Vector { x: r, y: _ } = mat.map_vector((r, 0.0));
                    Shape::Circle { cx, cy, r }
                }
                "text" => {
                    let (x, y) = (
                        svg::attribute_f32(node, "x")?,
                        svg::attribute_f32(node, "y")?,
                    );
                    let Point { x, y } = mat.map_point((x, y));
                    Shape::Text {
                        x,
                        y,
                        content: node
                            .children()
                            .map(|c| c.text().unwrap_or(""))
                            .collect::<Vec<_>>()
                            .join(""),
                    }
                }
                _ => unreachable!("unexpected SVG element: {}", node.tag_name().name()),
            },
        })
    }

    pub fn get_position(&self) -> (f32, f32) {
        match &self.shape {
            Shape::Path { d } => {
                log::warn!("position may not be accurate for path elements");
                let Point { x, y } = d.bounds().center();
                (x, y)
            }
            Shape::Circle { cx, cy, .. } => (*cx, *cy),
            Shape::Text { x, y, .. } => {
                log::warn!("position may not be accurate for text elements");
                (*x, *y)
            }
        }
    }
}
