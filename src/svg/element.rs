use std::path::PathBuf;

use roxmltree::Node;
use skia_safe::{Matrix, Path, Point, Vector, utils::text_utils::Align};

use crate::{animation::properties::Properties, svg, util::ChronuxResult};

#[derive(Debug)]
pub struct Object {
    pub id: String,
    pub style: Properties,
    pub shape: Shape,
}

#[derive(Debug)]
pub enum Shape {
    Path {
        d: Path,
    },
    Circle {
        cx: f32,
        cy: f32,
        r: f32,
    },
    Text {
        x: f32,
        y: f32,
        content: Vec<String>,
        align: Align,
    },
    Image {
        href: String,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
}

impl Object {
    pub fn from_node(node: &Node, id: String, mat: Matrix, base: PathBuf) -> ChronuxResult<Object> {
        let mut style = Properties::from_style(&svg::attribute_str(node, "style")?, mat)?;
        let shape = match node.tag_name().name() {
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

                let child = node.children().nth(0).unwrap();
                let child_style =
                    Properties::from_style(&child.attribute("style").unwrap_or(""), mat)?;

                if let Some(ff) = get_node_font_family(node) {
                    style.font_family = ff;
                }

                let content = get_node_text(node);

                let align = child_style.text_anchor;
                Shape::Text {
                    x,
                    y,
                    content,
                    align,
                }
            }
            "image" => {
                let href = base
                    .join(svg::attribute_namespace_str(
                        node,
                        "http://www.w3.org/1999/xlink",
                        "href",
                    )?)
                    .to_string_lossy()
                    .into_owned();
                let (x, y) = (
                    svg::attribute_f32(node, "x")?,
                    svg::attribute_f32(node, "y")?,
                );
                let (width, height) = (
                    svg::attribute_f32(node, "width")?,
                    svg::attribute_f32(node, "height")?,
                );
                let Point { x, y } = mat.map_point((x, y));
                let Vector {
                    x: width,
                    y: height,
                } = mat.map_vector((width, height));
                Shape::Image {
                    href,
                    x,
                    y,
                    width,
                    height,
                }
            }
            _ => unreachable!("unexpected SVG element: {}", node.tag_name().name()),
        };
        Ok(Object { id, style, shape })
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
            Shape::Image { x, y, .. } => {
                log::warn!("position may not be accurate for image elements");
                (*x, *y)
            }
        }
    }
}

fn get_node_text(node: &Node) -> Vec<String> {
    node.descendants()
        .filter_map(|n| {
            let s = if n.is_text() {
                n.text()
            } else {
                None
            };
            s.map(str::trim).map(str::to_string)
        })
        .collect::<Vec<_>>()
}

fn get_node_font_family(node: &Node) -> Option<String> {
    for n in node.descendants() {
        if n.text().is_some() && !n.text().unwrap().trim().is_empty() {
            if let Some(ff) = n.attribute("font-family") {
                println!("Found font-family: {}", ff);
                return Some(ff.to_string());
            }
        }
    }
    None
}
