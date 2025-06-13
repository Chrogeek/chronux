use std::collections::HashSet;

use roxmltree::{Document, Node};

use crate::{
    svg::element::Object,
    util::{ChronuxError, ChronuxResult},
};

pub mod element;

pub struct Svg {
    tree: Document<'static>,
}

impl Svg {
    pub fn from_file(filename: &str) -> ChronuxResult<Svg> {
        let svg_text = std::fs::read_to_string(filename).map_err(ChronuxError::IoError)?;
        // Leak the String to get a &'static str for Document
        let leaked_svg_text: &'static str = Box::leak(svg_text.clone().into_boxed_str());
        let svg = Document::parse(leaked_svg_text).map_err(ChronuxError::XmlError)?;
        Ok(Svg { tree: svg })
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
