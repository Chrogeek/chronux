use std::collections::HashSet;

use roxmltree::{Document, Node};

use crate::{
    svg::element::Object,
    util::{ChronuxError, ChronuxResult},
};

pub mod element;

pub struct Svg {
    tree: Document<'static>,
    width: f64,
    height: f64,
    viewbox: (f64, f64, f64, f64),
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

        let width = attribute_f64(&root, "width")?;
        let height = attribute_f64(&root, "height")?;

        let vb = attribute_str(&root, "viewBox")?
            .split_whitespace()
            .map(|s| s.parse::<f64>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(ChronuxError::ParseFloatError)?;
        if vb.len() != 4 {
            return Err(ChronuxError::SvgError(
                "viewBox must have exactly 4 values".into(),
            ));
        }
        let vb = (vb[0], vb[1], vb[2], vb[3]);
        if width <= 0.0 || height <= 0.0 {
            return Err(ChronuxError::SvgError(format!("invalid SVG dimension: width = {}, height = {}", width, height )));
        }
        if vb.2 <= 0.0 || vb.3 <= 0.0 {
            return Err(ChronuxError::SvgError(format!("invalid SVG viewbox: width = {}, height = {}", vb.2, vb.3)));
        }

        if ((vb.2 / width) / (vb.3 / height) - 1.0).abs() > 1e-6 {
            log::warn!(
                "SVG viewbox aspect ratio does not match width/height: viewBox = {:?}, width = {}, height = {}",
                vb, width, height
            );
        }

        Ok(Svg {
            tree: svg,
            width,
            height,
            viewbox: vb,
        })
    }

    pub fn search_labels(&self, labels: &HashSet<String>) -> ChronuxResult<Vec<Object>> {
        fn search_labels_dfs(
            node: &Node,
            labels: &HashSet<String>,
            parent: &Option<String>,
            results: &mut Vec<Object>,
        ) -> ChronuxResult<()> {
            let this_label = attribute_label(node);
            if let Some(ref label) = this_label {
                if labels.contains(label) {
                    results.push(Object::from_node(node, label.clone())?);
                }

                if let Some(parent_label) = parent {
                    let full_name = format!("{}/{}", parent_label, label);
                    if labels.contains(&full_name) {
                        results.push(Object::from_node(node, full_name)?);
                    }
                }
            }

            for child in node.children() {
                search_labels_dfs(&child, labels, &this_label, results)?;
            }

            Ok(())
        }

        let mut results = Vec::new();
        search_labels_dfs(&self.tree.root(), labels, &None, &mut results)?;

        Ok(results)
    }

    pub fn to_viewbox_coords(&self, x: f64, y: f64) -> (f64, f64) {
        let (vb_x, vb_y, vb_width, vb_height) = self.viewbox;
        let scale_x = self.width / vb_width;
        let scale_y = self.height / vb_height;

        let viewbox_x = (x - vb_x) * scale_x;
        let viewbox_y = (y - vb_y) * scale_y;

        (viewbox_x, viewbox_y)
    }

    pub fn to_viewbox_length(&self, length: f64) -> f64 {
        let (_, _, vb_width, vb_height) = self.viewbox;
        let scale = self.width / vb_width; // Assuming uniform scaling for both dimensions
        length * scale
    }

    pub fn from_viewbox_coords(&self, x: f64, y: f64) -> (f64, f64) {
        let (vb_x, vb_y, vb_width, vb_height) = self.viewbox;
        let scale_x = vb_width / self.width;
        let scale_y = vb_height / self.height;

        let svg_x = x * scale_x + vb_x;
        let svg_y = y * scale_y + vb_y;

        (svg_x, svg_y)
    }

    pub fn from_viewbox_length(&self, length: f64) -> f64 {
        let (_, _, vb_width, vb_height) = self.viewbox;
        let scale = vb_width / self.width; // Assuming uniform scaling for both dimensions
        length * scale
    }
}

pub fn attribute_f64(node: &Node, attribute: &str) -> ChronuxResult<f64> {
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
