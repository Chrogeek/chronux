use std::collections::HashMap;

use crate::svg::element::Shape;

#[derive(Clone, Debug)]
pub struct RenderDict {
    // shape-id => shape
    pub shapes: HashMap<String, Shape>,
    // path-id => path length
    pub path_lengths: HashMap<String, f32>,
}
