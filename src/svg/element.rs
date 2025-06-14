use roxmltree::Node;
use skia_safe::Path;

use crate::{
    animation::properties::Properties,
    svg::{self, Dimension},
    util::ChronuxResult,
};

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
    pub fn from_node(node: &Node, id: String, dim: &Dimension) -> ChronuxResult<Object> {
        let style = Properties::from_style(&svg::attribute_str(node, "style")?, dim)?;
        Ok(Object {
            id,
            style,
            shape: match node.tag_name().name() {
                "path" => Shape::Path {
                    d: svg::path::svg_to_path(&svg::attribute_str(node, "d")?, dim)?,
                },
                "circle" => {
                    let (cx, cy, r) = (
                        svg::attribute_f32(node, "cx")?,
                        svg::attribute_f32(node, "cy")?,
                        svg::attribute_f32(node, "r")?,
                    );
                    let (cx, cy) = dim.to_real_coords(cx, cy);
                    let r = dim.to_real_length(r);
                    Shape::Circle { cx, cy, r }
                }
                "text" => {
                    let (x, y) = (
                        svg::attribute_f32(node, "x")?,
                        svg::attribute_f32(node, "y")?,
                    );
                    let (x, y) = dim.to_real_coords(x, y);
                    Shape::Text {
                        x,
                        y,
                        content: node.text().unwrap_or_default().into(),
                    }
                }
                _ => unreachable!("unexpected SVG element: {}", node.tag_name().name()),
            },
        })
    }
}
