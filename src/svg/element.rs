use roxmltree::Node;

use crate::{svg, util::ChronuxResult};

#[derive(Debug)]
pub struct Object {
    pub id: String,
    pub style: String,
    pub shape: Shape,
}

#[derive(Debug)]
pub enum Shape {
    Path { d: String },
    Circle { cx: f64, cy: f64, r: f64 },
    Text { x: f64, y: f64, content: String },
}

impl Object {
    pub fn from_node(node: &Node, id: String) -> ChronuxResult<Object> {
        let style = svg::attribute_str(node, "style")?;
        Ok(Object {
            id,
            style,
            shape: match node.tag_name().name() {
                "path" => Shape::Path {
                    d: svg::attribute_str(node, "d")?,
                },
                "circle" => Shape::Circle {
                    cx: svg::attribute_f64(node, "cx")?,
                    cy: svg::attribute_f64(node, "cy")?,
                    r: svg::attribute_f64(node, "cr")?,
                },
                "text" => Shape::Text {
                    x: svg::attribute_f64(node, "x")?,
                    y: svg::attribute_f64(node, "y")?,
                    content: node.text().unwrap_or_default().into(),
                },
                _ => unreachable!("unexpected SVG element: {}", node.tag_name().name()),
            }
        })
    }
}
