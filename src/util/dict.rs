use std::collections::HashMap;

use crate::svg::element::Shape;

#[derive(Clone, Debug)]
pub struct RenderDict {
    pub shape_dict: HashMap<String, Shape>,
    pub path_length_dict: HashMap<String, f32>,
}
