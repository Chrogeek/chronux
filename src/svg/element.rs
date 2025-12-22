use std::path::PathBuf;

use roxmltree::Node;
use skia_safe::{Data, Image, Matrix, Path, PathMeasure, Point, Rect, Vector};
use urlencoding::decode;

use crate::{
    animation::properties::Properties,
    svg::{
        self, attribute_label, attribute_transform, placeholder_text::PlaceholderText, tspan::Tspan,
    },
    util::{ChronuxError, ChronuxResult},
};

#[derive(Debug, Clone)]
pub struct Object {
    pub id: String,
    pub style: Properties,
    pub shape: Shape,
    pub apply_transform: bool,
    pub transform: Matrix,
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
    Rect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        rx: f32,
        ry: f32,
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
        let style = Properties::from_style(
            &svg::attribute_str(node, "style").unwrap_or_default(),
            mat,
            Default::default(),
        )?;
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
                let r = mat.map_vector((r, 0.0)).length();
                Shape::Circle { cx, cy, r }
            }
            "rect" => {
                let (x, y, width, height, rx, ry) = (
                    svg::attribute_f32(node, "x")?,
                    svg::attribute_f32(node, "y")?,
                    svg::attribute_f32(node, "width")?,
                    svg::attribute_f32(node, "height")?,
                    match svg::attribute_f32(node, "rx") {
                        Ok(v) => v,
                        Err(ChronuxError::AttributeNotFound { .. }) => 0.0,
                        Err(e) => return Err(e),
                    },
                    match svg::attribute_f32(node, "ry") {
                        Ok(v) => v,
                        Err(ChronuxError::AttributeNotFound { .. }) => 0.0,
                        Err(e) => return Err(e),
                    },
                );
                let Point { x, y } = mat.map_point((x, y));
                let width = mat.map_vector((width, 0.0)).length();
                let height = mat.map_vector((0.0, height)).length();
                let rx = mat.map_vector((rx, 0.0)).length();
                let ry = mat.map_vector((0.0, ry)).length();
                Shape::Rect {
                    x,
                    y,
                    width,
                    height,
                    rx,
                    ry,
                }
            }
            "text" => {
                let (x, y) = (
                    svg::attribute_f32(node, "x")?,
                    svg::attribute_f32(node, "y")?,
                );
                let Point { x, y } = mat.map_point((x, y));
                let placeholder = node
                    .attribute("chronux-placeholder")
                    .and_then(PlaceholderText::parse);
                let children = Tspan::from_subtree(node, mat, x, y, style.clone(), &placeholder)?;
                Shape::Text { x, y, children }
            }
            "image" => {
                let href =
                    svg::attribute_namespace_str(node, "http://www.w3.org/1999/xlink", "href")?;
                let href = base
                    .join(
                        decode(&href)
                            .map_err(|e| {
                                ChronuxError::HrefError(format!(
                                    "Failed to decode URL {}: {}",
                                    href, e
                                ))
                            })?
                            .to_string(),
                    )
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
                    "Failed to load image data from {href}"
                )))?;
                let img = Image::from_encoded(data).ok_or(ChronuxError::HrefError(format!(
                    "Failed to load image from {href}"
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
            }
            _ => unreachable!("unexpected SVG element: {}", node.tag_name().name()),
        };
        Ok(Object {
            id,
            style,
            shape,
            apply_transform,
            transform: mat,
        })
    }

    pub fn get_position(&self, properties: &Properties) -> (f32, f32) {
        let (tx, ty) = (properties.translate_x, properties.translate_y);
        match &self.shape {
            Shape::Path { d } => {
                log::warn!(
                    "position may not be accurate for path element '{}'",
                    self.id
                );
                let Point { x, y } = d.bounds().center();
                (x + tx, y + ty)
            }
            Shape::Circle { cx, cy, .. } => (*cx + tx, *cy + ty),
            Shape::Rect {
                x,
                y,
                width,
                height,
                ..
            } => (*x + width / 2.0 + tx, *y + height / 2.0 + ty),
            Shape::Text { x, y, .. } => {
                log::warn!(
                    "position may not be accurate for text element '{}'",
                    self.id
                );
                (*x + tx, *y + ty)
            }
            Shape::Image { x, y, .. } => {
                log::warn!(
                    "position may not be accurate for image element '{}'",
                    self.id
                );
                (*x + tx, *y + ty)
            }
            Shape::Group { objects } => {
                log::warn!(
                    "position may not be accurate for group element '{}'",
                    self.id
                );
                if let Some(first) = objects.first() {
                    first.get_position(properties)
                } else {
                    (0.0, 0.0)
                }
            }
        }
    }

    pub fn get_bounds(&self, properties: &Properties) -> Rect {
        match &self.shape {
            Shape::Path { d } => {
                let mut bounds = *d.bounds();
                bounds.offset((properties.translate_x, properties.translate_y));
                bounds
            }
            Shape::Circle { cx, cy, r } => Rect::new(
                *cx - *r + properties.translate_x,
                *cy - *r + properties.translate_y,
                *cx + *r,
                *cy + *r,
            ),
            Shape::Rect {
                x,
                y,
                width,
                height,
                ..
            } => Rect::from_xywh(
                *x + properties.translate_x,
                *y + properties.translate_y,
                *width,
                *height,
            ),
            Shape::Text { children, .. } => {
                let mut bounds = children
                    .iter()
                    .map(|c| c.bounds)
                    .reduce(|mut a, b| {
                        a.join(b);
                        a
                    })
                    .unwrap_or(Rect::new_empty());
                bounds.offset((properties.translate_x, properties.translate_y));
                bounds
            }
            Shape::Image {
                x,
                y,
                width,
                height,
                ..
            } => Rect::from_xywh(
                *x + properties.translate_x,
                *y + properties.translate_y,
                *width,
                *height,
            ),
            Shape::Group { objects } => objects
                .iter()
                .map(|c| c.get_bounds(properties))
                .reduce(|mut a, b| {
                    a.join(b);
                    a
                })
                .unwrap_or(Rect::new_empty()),
        }
    }

    pub fn get_length(&self) -> Option<f32> {
        match &self.shape {
            Shape::Path { d } => {
                let mut pm = PathMeasure::new(d, false, Some(2.0));
                Some(pm.length())
            }
            _ => None,
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
            "path" | "circle" | "text" | "image" | "rect" => {
                results.push(Object::from_node(
                    node,
                    format!(
                        "group-element-{}/{}",
                        base_id,
                        attribute_label(node).unwrap_or("unknown".to_string())
                    ),
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
        let element_mat = attribute_transform(&child);
        let mat = matrix * element_mat;
        flatten_group_dfs(&child, results, mat, base.clone(), base_id, apply_transform)?;
    }

    Ok(())
}
