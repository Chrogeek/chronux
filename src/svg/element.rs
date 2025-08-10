use std::path::PathBuf;

use roxmltree::Node;
use skia_safe::{Data, Image, Matrix, Path, Point, Vector};

use crate::{
    animation::properties::Properties,
    svg::{self, attribute_label, attribute_transform, tspan::Tspan},
    util::{ChronuxError, ChronuxResult},
};

#[derive(Debug, Clone)]
pub struct Object {
    pub id: String,
    pub style: Properties,
    pub shape: Shape,
    pub apply_transform: bool,
}

#[derive(Debug, Clone)]
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
        children: Vec<Tspan>,
    },
    Image {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        skia_img: Image,
    },
    Group {
        objects: Vec<Object>,
    },
}

impl Object {
    pub fn from_node(
        node: &Node,
        id: String,
        mat: Matrix,
        base: PathBuf,
        apply_transform: bool,
    ) -> ChronuxResult<Object> {
        let style =
            Properties::from_style(&svg::attribute_str(node, "style").unwrap_or(String::new()), mat, Default::default())?;
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
                let children = Tspan::from_subtree(&node, mat, x, y, style.clone())?;
                Shape::Text { x, y, children }
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

                let data = Data::from_filename(&href).ok_or(ChronuxError::HrefError(format!(
                    "Failed to load image data from {}",
                    href
                )))?;
                let img = Image::from_encoded(data).ok_or(ChronuxError::HrefError(format!(
                    "Failed to load image from {}",
                    href
                )))?;
                Shape::Image {
                    x,
                    y,
                    width,
                    height,
                    skia_img: img,
                }
            }
            "g" => {
                let mut objects = Vec::new();
                flatten_group_dfs(node, &mut objects, mat, base.clone(), &id, apply_transform)?;
                Shape::Group { objects }
            },
            _ => unreachable!("unexpected SVG element: {}", node.tag_name().name()),
        };
        Ok(Object {
            id,
            style,
            shape,
            apply_transform,
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
            Shape::Image { x, y, .. } => {
                log::warn!("position may not be accurate for image elements");
                (*x, *y)
            }
            Shape::Group { objects } => {
                log::warn!("position may not be accurate for image elements");
                if let Some(first) = objects.first() {
                    first.get_position()
                } else {
                    (0.0, 0.0)
                }
            }
        }
    }
}

fn flatten_group_dfs(
    node: &Node,
    results: &mut Vec<Object>,
    matrix: Matrix,
    base: PathBuf,
    base_id: &String,
    apply_transform: bool,
) -> ChronuxResult<()> {
    if node.is_element() {
        match node.tag_name().name() {
            "path" | "circle" | "text" | "image" => {
                results.push(Object::from_node(
                    node,
                    format!("group-element-{}/{}", base_id, attribute_label(node).unwrap_or("unknown".to_string())),
                    matrix,
                    base.clone(),
                    apply_transform,
                )?);
            }
            "g" => (),
            _ => (),
        }
    }

    for child in node.children() {
        let mat = matrix * attribute_transform(&child);
        flatten_group_dfs(&child, results, mat, base.clone(), base_id, apply_transform)?;
    }

    Ok(())
}
