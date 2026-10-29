use std::path::PathBuf;

use roxmltree::Node;
use skia_safe::{Data, Image, Matrix, Path, PathMeasure, Point, Rect, Vector};
use urlencoding::decode;

use crate::{
    animation::properties::Properties,
    svg::{
        self, attribute_label, attribute_transform, placeholder_text::PlaceholderText, tspan::Tspan,
    },
    util::error::{ChronuxError, ChronuxResult},
};

#[derive(Debug, Clone)]
pub struct Object {
    pub id: String,
    pub style: Properties,
    pub apply_transform: bool,
    pub transform: Matrix,
    pub mask: Option<Mask>,
}

#[derive(Debug, Clone)]
pub enum Mask {
    Fill(String),
    Stroke(String),
}

#[derive(Debug, Clone)]
pub enum Shape {
    None,
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
        objects: Vec<Properties>,
    },
}

impl Shape {
    pub fn get_bounds(&self) -> Rect {
        match self {
            Shape::None => Rect::new_empty(),
            Shape::Path { d } => *d.bounds(),
            Shape::Circle { cx, cy, r } => Rect::from_xywh(*cx - *r, *cy - *r, *r * 2.0, *r * 2.0),
            Shape::Rect {
                x,
                y,
                width,
                height,
                ..
            } => Rect::from_xywh(*x, *y, *width, *height),
            Shape::Text { children, .. } => children
                .iter()
                .map(|c| c.preshaped.bounds)
                .reduce(|mut a, b| {
                    a.join(b);
                    a
                })
                .unwrap_or(Rect::new_empty()),
            Shape::Image {
                x,
                y,
                width,
                height,
                ..
            } => Rect::from_xywh(*x, *y, *width, *height),
            Shape::Group { objects } => objects
                .iter()
                .map(|c| c.shape.get_bounds())
                .reduce(|mut a, b| {
                    a.join(b);
                    a
                })
                .unwrap_or(Rect::new_empty()),
        }
    }

    pub fn get_position(&self) -> Point {
        match self {
            Shape::None => Point::new(0.0, 0.0),
            Shape::Circle { cx, cy, .. } => Point::new(*cx, *cy),
            Shape::Rect {
                x,
                y,
                width,
                height,
                ..
            }
            | Shape::Image {
                x,
                y,
                width,
                height,
                ..
            } => Rect::from_xywh(*x, *y, *width, *height).center(),
            Shape::Text { x, y, .. } => Point::new(*x, *y),
            _ => self.get_bounds().center(),
        }
    }
}

impl Object {
    pub fn from_node(
        node: &Node,
        id: String,
        mat: Matrix,
        base: PathBuf,
        apply_transform: bool,
    ) -> ChronuxResult<Object> {
        let mask = svg::attribute_str(node, "chronux-mask-mode")
            .map(|s| Mask::try_from(s))
            .flatten()
            .ok();

        let blur = svg::attribute_f32(node, "chronux-blur").ok();
        let shadow = svg::attribute_f32(node, "chronux-shadow").ok();
        let small_fraction = svg::attribute_f32(node, "chronux-small-fraction").unwrap_or(1.0);

        let style = Properties::from_style(
            &svg::attribute_str(node, "style").unwrap_or_default(),
            mat,
            Default::default(),
        )?;
        let style = Properties {
            blur,
            shadow,
            small_fraction,
            ..style
        };

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
                let unshaped = node
                    .attribute("chronux-text-unshaped")
                    .map(|s| s.parse::<u32>().ok())
                    .flatten()
                    .map(|u| u != 0)
                    .unwrap_or(false);
                let children =
                    Tspan::from_subtree(node, mat, x, y, style.clone(), &placeholder, unshaped)?;
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
        let style = Properties { shape, ..style };
        Ok(Object {
            mask,
            id,
            style,
            apply_transform,
            transform: mat,
        })
    }

    pub fn get_position(&self, properties: &Properties) -> (f32, f32) {
        // TODO: 暂时还没支持 scale, rotate 等变换
        let Point { x, y } = self.style.shape.get_position();
        let (tx, ty) = (properties.translate_x, properties.translate_y);
        (x + tx, y + ty)
    }

    pub fn get_bounds(&self, properties: &Properties) -> Rect {
        // TODO: 暂时还没支持 scale, rotate 等变换
        let raw = self.style.shape.get_bounds();
        let raw = raw.with_offset((properties.translate_x, properties.translate_y));
        raw
    }

    pub fn get_length(&self) -> Option<f32> {
        match &self.style.shape {
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
    results: &mut Vec<Properties>,
    matrix: Matrix,
    base: PathBuf,
    base_id: &String,
    apply_transform: bool,
) -> ChronuxResult<()> {
    if node.is_element() {
        match node.tag_name().name() {
            "path" | "circle" | "text" | "image" | "rect" => {
                let obj = Object::from_node(
                    node,
                    format!(
                        "group-element-{}/{}",
                        base_id,
                        attribute_label(node).unwrap_or("unknown".to_string())
                    ),
                    matrix,
                    base.clone(),
                    apply_transform,
                )?;
                results.push(obj.style);
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

impl TryFrom<String> for Mask {
    type Error = ChronuxError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        // value should be either "stroke(<id>)" or "fill(<id>)"
        if let Some(id) = value
            .strip_prefix("stroke(")
            .and_then(|s| s.strip_suffix(')'))
        {
            Ok(Mask::Stroke(id.to_string()))
        } else if let Some(id) = value
            .strip_prefix("fill(")
            .and_then(|s| s.strip_suffix(')'))
        {
            Ok(Mask::Fill(id.to_string()))
        } else {
            Err(ChronuxError::ArgError(format!(
                "Invalid mask string: {}",
                value
            )))
        }
    }
}
