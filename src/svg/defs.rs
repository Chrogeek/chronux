use roxmltree::Node;
use skia_safe::{Color, Matrix, Point};

use crate::{
    svg::{attribute_f32, attribute_namespace_str, attribute_str, parse_transform},
    util::parse_css_color,
};

#[derive(Debug, Clone)]
pub enum SvgDef {
    LinearGradient {
        p1: Option<Point>,
        p2: Option<Point>,
        stops: Vec<SvgGradientStop>,
        href: Option<String>,
        gradient_transform: Matrix,
    },
    GaussianBlur {
        std_deviation: f32,
    },
}

#[derive(Debug, Clone)]
pub struct SvgGradientStop {
    pub offset: f32,
    pub color: Color,
}

pub fn get_def_node(node: &Node, matrix: &Matrix) -> Option<(String, SvgDef)> {
    match node.tag_name().name() {
        "linearGradient" => {
            let id = attribute_str(node, "id").unwrap_or_default();
            let x1 = attribute_f32(node, "x1").ok();
            let y1 = attribute_f32(node, "y1").ok();
            let x2 = attribute_f32(node, "x2").ok();
            let y2 = attribute_f32(node, "y2").ok();
            let href = attribute_namespace_str(node, "http://www.w3.org/1999/xlink", "href").ok();

            let gradient_transform = attribute_str(node, "gradientTransform").unwrap_or_default();
            let matrix = if !gradient_transform.is_empty() {
                *matrix * parse_transform(&gradient_transform).unwrap_or(Matrix::new_identity())
            } else {
                *matrix
            };

            let p1 = if let Some(x1) = x1
                && let Some(y1) = y1
            {
                Some(Point::new(x1, y1))
            } else {
                None
            };

            let p2 = if let Some(x2) = x2
                && let Some(y2) = y2
            {
                Some(Point::new(x2, y2))
            } else {
                None
            };

            let mut stops = Vec::new();
            for child in node.children().filter(|n| n.is_element()) {
                if child.tag_name().name() == "stop" {
                    let offset = attribute_f32(&child, "offset").unwrap_or(0.0);
                    let style = attribute_str(&child, "style").unwrap_or_default();
                    let mut color = String::new();
                    let mut opacity = 1.0;

                    for part in style.split(';') {
                        let mut kv = part.splitn(2, ':');
                        let key = kv.next().unwrap_or("").trim();
                        let value = kv.next().unwrap_or("").trim();
                        match key {
                            "stop-color" => color = value.to_string(),
                            "stop-opacity" => {
                                opacity = value.parse().unwrap_or(1.0);
                            }
                            _ => {}
                        }
                    }

                    stops.push(SvgGradientStop {
                        offset,
                        color: parse_css_color(&color, opacity).ok()?,
                    });
                }
            }

            Some((
                id,
                SvgDef::LinearGradient {
                    p1,
                    p2,
                    stops,
                    href,
                    gradient_transform: matrix,
                },
            ))
        }
        "filter" => {
            let id = attribute_str(node, "id").unwrap_or_default();
            let mut std_deviation = 0.0;

            for child in node.children().filter(|n| n.is_element()) {
                if child.tag_name().name() == "feGaussianBlur" {
                    std_deviation = attribute_f32(&child, "stdDeviation").unwrap_or(0.0);
                }
            }

            Some((id, SvgDef::GaussianBlur { std_deviation }))
        }
        _ => {
            // TODO
            log::warn!("Unsupported def type: {}", node.tag_name().name());
            None
        }
    }
}
