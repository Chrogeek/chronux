use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use roxmltree::{Document, Node};
use skia_safe::{FourByteTag, Matrix, shaper::Feature};
use svgtypes::{TransformListParser, TransformListToken};

use crate::{
    svg::{
        defs::{SvgDef, get_def_node},
        element::Object,
    },
    util::error::{ChronuxError, ChronuxResult},
};

pub mod defs;
pub mod element;
pub mod path;
pub mod placeholder_text;
pub mod tspan;

pub struct Svg {
    tree: Document<'static>,
    pub transform: Matrix,
}

impl Svg {
    pub fn from_file(filename: PathBuf) -> ChronuxResult<Svg> {
        let svg_text = std::fs::read_to_string(filename)?;
        // Leak the String to get a &'static str for Document
        let leaked_svg_text: &'static str = Box::leak(svg_text.clone().into_boxed_str());
        let svg = Document::parse(leaked_svg_text)?;

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
            .collect::<Result<Vec<_>, _>>()?;
        if vb.len() != 4 {
            return Err(ChronuxError::SvgError(
                "viewBox must have exactly 4 values".into(),
            ));
        }
        let vb = (vb[0], vb[1], vb[2], vb[3]);
        if width <= 0.0 || height <= 0.0 {
            return Err(ChronuxError::SvgError(format!(
                "invalid SVG dimension: width = {width}, height = {height}"
            )));
        }
        if vb.2 <= 0.0 || vb.3 <= 0.0 {
            return Err(ChronuxError::SvgError(format!(
                "invalid SVG viewbox: width = {}, height = {}",
                vb.2, vb.3
            )));
        }

        if ((vb.2 / width) / (vb.3 / height) - 1.0).abs() > 1e-4 {
            log::warn!(
                "SVG viewbox aspect ratio does not match width/height: viewBox = {vb:?}, width = {width}, height = {height}"
            );
        }

        Ok(Svg {
            tree: svg,
            transform: Matrix::scale((width / vb.2, height / vb.3))
                * Matrix::translate((vb.0, vb.1)),
        })
    }

    pub fn search_labels(
        &self,
        labels: &HashSet<String>,
        base: PathBuf,
        apply_transform: bool,
    ) -> ChronuxResult<Vec<Object>> {
        fn search_labels_dfs(
            node: &Node,
            labels: &HashSet<String>,
            parent: &Option<String>,
            results: &mut Vec<Object>,
            matrix: Matrix,
            base: PathBuf,
            apply_transform: bool,
        ) -> ChronuxResult<()> {
            let this_label = attribute_label(node);

            let element_mat = attribute_transform(node);
            let mat = matrix * element_mat;

            if let Some(ref label) = this_label {
                if labels.contains(label) {
                    results.push(Object::from_node(
                        node,
                        label.clone(),
                        mat,
                        base.clone(),
                        apply_transform,
                    )?);
                }

                if let Some(parent_label) = parent {
                    let full_name = format!("{parent_label}/{label}");
                    if labels.contains(&full_name) {
                        results.push(Object::from_node(
                            node,
                            full_name,
                            mat,
                            base.clone(),
                            apply_transform,
                        )?);
                    }
                }
            }

            for child in node.children() {
                search_labels_dfs(
                    &child,
                    labels,
                    &this_label,
                    results,
                    mat,
                    base.clone(),
                    apply_transform,
                )?;
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
            base,
            apply_transform,
        )?;

        Ok(results)
    }

    pub fn get_defs(&self) -> HashMap<String, SvgDef> {
        let mut defs = HashMap::new();

        fn get_defs_dfs(node: &Node, defs: &mut HashMap<String, SvgDef>, matrix: Matrix) {
            let mat = matrix * attribute_transform(node);

            if node.has_tag_name("defs") {
                for child in node.children().filter(|n| n.is_element()) {
                    if let Some((id, def)) = get_def_node(&child) {
                        defs.insert(id, def);
                    }
                }
            }

            for child in node.children() {
                get_defs_dfs(&child, defs, mat);
            }
        }

        get_defs_dfs(&self.tree.root(), &mut defs, self.transform);

        // 遍历所有 defs, 处理 href
        let mut new_defs = HashMap::new();
        for (id, def) in defs.iter() {
            match def {
                SvgDef::LinearGradient {
                    p1,
                    p2,
                    stops,
                    href,
                    gradient_transform,
                } => {
                    if let Some(href) = href {
                        let href_id = href.trim_start_matches('#');
                        if let Some(base_def) = defs.get(href_id) {
                            let SvgDef::LinearGradient {
                                p1: _,
                                p2: _,
                                stops: b_stops,
                                href: b_href,
                                ..
                            } = base_def;
                            let new_stops = if stops.is_empty() {
                                b_stops.clone()
                            } else {
                                stops.clone()
                            };
                            new_defs.insert(
                                id.clone(),
                                SvgDef::LinearGradient {
                                    p1: *p1,
                                    p2: *p2,
                                    stops: new_stops,
                                    href: b_href.clone(),
                                    gradient_transform: *gradient_transform,
                                },
                            );
                            continue;
                        }
                    }
                    // href 指向的不是线性渐变，或
                    // href 指向的 id 不存在，或
                    // 自己没有 href，直接插入原始定义
                    new_defs.insert(id.clone(), def.clone());
                }
            }
        }
        new_defs
    }
}

pub fn attribute_f32(node: &Node, attribute: &str) -> ChronuxResult<f32> {
    node.attribute(attribute)
        .ok_or(ChronuxError::AttributeNotFound {
            node_id: attribute_label(node),
            attribute: attribute.to_string(),
        })?
        .parse()
        .map_err(ChronuxError::ParseFloatError)
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

pub fn attribute_namespace_str(
    node: &Node,
    namespace: &str,
    attribute: &str,
) -> ChronuxResult<String> {
    Ok(node
        .attribute((namespace, attribute))
        .ok_or(ChronuxError::AttributeNotFound {
            node_id: attribute_label(node),
            attribute: format!("{namespace}:{attribute}"),
        })?
        .into())
}

pub fn attribute_label(node: &Node) -> Option<String> {
    node.attribute(("http://www.inkscape.org/namespaces/inkscape", "label"))
        .map(|s| s.to_string())
}

pub fn attribute_transform(node: &Node) -> Matrix {
    node.attribute("transform")
        .map(|s| {
            parse_transform(s).unwrap_or_else(|e| {
                log::warn!("Failed to parse transform '{}': {}", s, e);
                Matrix::new_identity()
            })
        })
        .unwrap_or_else(|| Matrix::new_identity())
}

pub fn parse_transform(transform: &str) -> ChronuxResult<Matrix> {
    let mut mat = Matrix::new_identity();
    for t in TransformListParser::from(transform) {
        let transform = t.map_err(|e| {
            ChronuxError::SvgError(format!("failed to parse transform '{transform}': {e}"))
        })?;
        let t = match transform {
            TransformListToken::Translate { tx, ty } => Matrix::translate((tx as f32, ty as f32)),
            TransformListToken::Scale { sx, sy } => Matrix::scale((sx as f32, sy as f32)),
            TransformListToken::Rotate { angle } => Matrix::rotate_deg(angle as f32),
            TransformListToken::SkewX { angle } => Matrix::skew((angle as f32, 0.0)),
            TransformListToken::SkewY { angle } => Matrix::skew((0.0, angle as f32)),
            TransformListToken::Matrix { a, b, c, d, e, f } => {
                Matrix::from_affine(&[a as f32, b as f32, c as f32, d as f32, e as f32, f as f32])
            }
        };
        mat = mat * t;
    }
    Ok(mat)
}

pub fn parse_font_features(style: &str, start: usize, end: usize) -> Vec<Feature> {
    let rules = style
        .split(';')
        .filter_map(|s| s.trim().split_once(':'))
        .map(|(k, v)| (k.trim(), v.trim()))
        .collect::<std::collections::HashMap<_, _>>();

    let mut features = HashMap::new();

    match rules.get("font-variant-numeric") {
        Some(&"lining-nums") => {
            features.insert("lnum".to_string(), 1);
        }
        Some(&"oldstyle-nums") => {
            features.insert("onum".to_string(), 1);
        }
        Some(&"proportional-nums") => {
            features.insert("pnum".to_string(), 1);
        }
        Some(&"tabular-nums") => {
            features.insert("tnum".to_string(), 1);
        }
        Some(&"diagonal-fractions") => {
            features.insert("frac".to_string(), 1);
        }
        Some(&"stacked-fractions") => {
            features.insert("afrc".to_string(), 1);
        }
        Some(&"ordinal") => {
            features.insert("ordn".to_string(), 1);
        }
        Some(&"slashed-zero") => {
            features.insert("zero".to_string(), 1);
        }
        _ => {}
    }

    match rules.get("font-variant-position") {
        Some(&"sub") => {
            features.insert("subs".to_string(), 1);
        }
        Some(&"super") => {
            features.insert("sups".to_string(), 1);
        }
        _ => {}
    }

    if let Some(f) = rules.get("font-feature-settings") {
        f.split(',')
            .filter_map(|s| {
                let s = s.trim();
                if s.is_empty() {
                    return None;
                }
                let mut parts = s.split_whitespace();
                let tag = parts.next()?;
                let tag = if tag.len() == 6
                    && (tag.starts_with("\"") || tag.starts_with("'"))
                    && (tag.ends_with("\"") || tag.ends_with("'"))
                {
                    tag[1..5].to_string()
                } else if tag.len() == 4 {
                    tag.to_string()
                } else {
                    return None;
                };
                let value = match parts.next() {
                    Some("on") => 1,
                    Some("off") => 0,
                    None => 1,
                    Some(x) => x.parse::<u32>().ok()?,
                };
                Some((tag, value))
            })
            .for_each(|(tag, value)| {
                features.insert(tag, value);
            });
    }

    features
        .into_iter()
        .filter_map(|(tag, value)| {
            let tag = tag.trim();
            if tag.len() != 4 {
                return None;
            }
            let tag = tag.chars().collect::<Vec<_>>();
            Some(Feature {
                tag: *FourByteTag::from_chars(tag[0], tag[1], tag[2], tag[3]),
                value,
                start,
                end,
            })
        })
        .collect()
}
