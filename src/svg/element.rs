use roxmltree::Node;

use crate::{svg, util::ChronuxResult};

#[derive(Debug)]
pub struct Obj {
    pub id: String,
    pub style: String,
    pub zorder: u32,
    pub shape: Object,
}

#[derive(Debug)]
pub enum Object {
    Path { d: String },
    Circle { cx: f64, cy: f64, r: f64 },
    Text { x: f64, y: f64, content: String },
}

impl Obj {
    pub fn from_node(node: &Node, id: String) -> ChronuxResult<Obj> {
        let style = svg::attribute_str(node, "style")?;
        Ok(Obj {
            id,
            style,
            zorder: 0,
            shape: match node.tag_name().name() {
                "path" => Object::Path {
                    d: svg::attribute_str(node, "d")?,
                },
                "circle" => Object::Circle {
                    cx: svg::attribute_f64(node, "cx")?,
                    cy: svg::attribute_f64(node, "cy")?,
                    r: svg::attribute_f64(node, "cr")?,
                },
                "text" => Object::Text {
                    x: svg::attribute_f64(node, "x")?,
                    y: svg::attribute_f64(node, "y")?,
                    content: node.text().unwrap_or_default().into(),
                },
                _ => unreachable!("unexpected SVG element: {}", node.tag_name().name()),
            }
        })
    }
}
