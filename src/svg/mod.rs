use std::collections::HashSet;

use roxmltree::{Document, Node};
use skia_safe::Matrix;
use svgtypes::{TransformListParser, TransformListToken};

use crate::{
    svg::element::Object,
    util::{ChronuxError, ChronuxResult},
};

pub mod element;
pub mod path;

pub struct Svg {
    tree: Document<'static>,
    pub transform: Matrix,
}

impl Svg {
    pub fn from_file(filename: &str) -> ChronuxResult<Svg> {
        let svg_text = std::fs::read_to_string(filename).map_err(ChronuxError::IoError)?;
        // Leak the String to get a &'static str for Document
        let leaked_svg_text: &'static str = Box::leak(svg_text.clone().into_boxed_str());
        let svg = Document::parse(leaked_svg_text).map_err(ChronuxError::XmlError)?;

        let root = svg
            .root_element()
            .descendants()
            .find(|n| n.has_tag_name("svg"))
            .ok_or(ChronuxError::SvgError("missing SVG root element".into()))?;

        let width = attribute_f32(&root, "width")?;
        let height = attribute_f32(&root, "height")?;

        let vb = attribute_str(&root, "viewBox")?
            .split_whitespace()
            .map(|s| s.parse::<f32>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(ChronuxError::ParseFloatError)?;
        if vb.len() != 4 {
            return Err(ChronuxError::SvgError(
                "viewBox must have exactly 4 values".into(),
            ));
        }
        let vb = (vb[0], vb[1], vb[2], vb[3]);
        if width <= 0.0 || height <= 0.0 {
            return Err(ChronuxError::SvgError(format!(
                "invalid SVG dimension: width = {}, height = {}",
                width, height
            )));
        }
        if vb.2 <= 0.0 || vb.3 <= 0.0 {
            return Err(ChronuxError::SvgError(format!(
                "invalid SVG viewbox: width = {}, height = {}",
                vb.2, vb.3
            )));
        }

        if ((vb.2 / width) / (vb.3 / height) - 1.0).abs() > 1e-6 {
            log::warn!(
                "SVG viewbox aspect ratio does not match width/height: viewBox = {:?}, width = {}, height = {}",
                vb,
                width,
                height
            );
        }

        Ok(Svg {
            tree: svg,
            transform: Matrix::scale((width / vb.2, height / vb.3))
                * Matrix::translate((vb.0, vb.1)),
        })
    }

    pub fn search_labels(&self, labels: &HashSet<String>) -> ChronuxResult<Vec<Object>> {
        fn search_labels_dfs(
            node: &Node,
            labels: &HashSet<String>,
            parent: &Option<String>,
            results: &mut Vec<Object>,
            matrix: Matrix,
        ) -> ChronuxResult<()> {
            let this_label = attribute_label(node);
            if let Some(ref label) = this_label {
                if labels.contains(label) {
                    results.push(Object::from_node(node, label.clone(), matrix)?);
                }

                if let Some(parent_label) = parent {
                    let full_name = format!("{}/{}", parent_label, label);
                    if labels.contains(&full_name) {
                        results.push(Object::from_node(node, full_name, matrix)?);
                    }
                }
            }

            let mat = node
                .attribute("transform")
                .iter()
                .flat_map(|s| {
                    TransformListParser::from(*s).map(|t| {
                        if let Ok(transform) = t {
                            Some(match transform {
                                TransformListToken::Translate { tx, ty } => {
                                    Matrix::translate((tx as f32, ty as f32))
                                }
                                TransformListToken::Scale { sx, sy } => {
                                    Matrix::scale((sx as f32, sy as f32))
                                }
                                TransformListToken::Rotate { angle } => {
                                    Matrix::rotate_deg(angle as f32)
                                }
                                TransformListToken::SkewX { angle } => {
                                    Matrix::skew((angle as f32, 0.0))
                                }
                                TransformListToken::SkewY { angle } => {
                                    Matrix::skew((0.0, angle as f32))
                                }
                                TransformListToken::Matrix { a, b, c, d, e, f } => {
                                    Matrix::from_affine(&[
                                        a as f32, b as f32, c as f32, d as f32, e as f32, f as f32,
                                    ])
                                }
                            })
                        } else {
                            None
                        }
                    })
                })
                .filter_map(|t| t)
                .fold(Matrix::new_identity(), |acc, t| acc * t);

            for child in node.children() {
                search_labels_dfs(&child, labels, &this_label, results, matrix * mat)?;
            }

            Ok(())
        }

        let mut results = Vec::new();
        search_labels_dfs(
            &self.tree.root(),
            labels,
            &None,
            &mut results,
            self.transform,
        )?;

        Ok(results)
    }
}

pub fn attribute_f32(node: &Node, attribute: &str) -> ChronuxResult<f32> {
    node.attribute(attribute)
        .ok_or(ChronuxError::AttributeNotFound {
            node_id: attribute_label(node),
            attribute: attribute.to_string(),
        })?
        .parse()
        .map_err(|e| ChronuxError::ParseFloatError(e))
}

pub fn attribute_str(node: &Node, attribute: &str) -> ChronuxResult<String> {
    Ok(node
        .attribute(attribute)
        .ok_or(ChronuxError::AttributeNotFound {
            node_id: attribute_label(node),
            attribute: attribute.to_string(),
        })?
        .into())
}

pub fn attribute_label(node: &Node) -> Option<String> {
    node.attribute(("http://www.inkscape.org/namespaces/inkscape", "label"))
        .map(|s| s.to_string())
}
