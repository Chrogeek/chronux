use std::collections::HashMap;

use serde::Deserialize;

use crate::{
    animation::{properties::Properties, track_on_path},
    svg::element::{Object, Shape},
    util::{ChronuxError, ChronuxResult},
};

#[derive(Debug, Deserialize, Clone)]
pub struct ElevatedConfig {
    pub gap: f32,
    #[serde(rename = "stroke-width")]
    pub stroke_width: f32,
    #[serde(rename = "gap-color")]
    pub gap_color: String,
    pub points: HashMap<String, Vec<String>>,
}

impl ElevatedConfig {
    pub fn parse_points(&self, objs: &[Object]) -> ChronuxResult<HashMap<String, Vec<(f32, f32)>>> {
        let obj_map: HashMap<String, &Object> = objs.iter().map(|o| (o.id.clone(), o)).collect();
        self.points
            .iter()
            .map(|(key, values)| {
                match &obj_map
                    .get(key)
                    .ok_or(ChronuxError::ArgError(format!(
                        "No entry found for key '{key}' in [drawing.elevated]"
                    )))?
                    .style
                    .shape
                {
                    Shape::Path { d } => {
                        let mut points = values
                            .iter()
                            .map(|p| {
                                if p == "" {
                                    0.0
                                } else {
                                    let (x, y) = obj_map[p].get_position(&Properties::default());
                                    track_on_path(d, x, y).0
                                }
                            })
                            .collect::<Vec<_>>();
                        if points.len() % 2 == 1 {
                            points.push(1.0);
                        }
                        let intervals =
                            points.chunks(2).map(|chunk| (chunk[0], chunk[1])).collect();
                        Ok((key.clone(), intervals))
                    }
                    _ => Err(ChronuxError::ArgError(format!(
                        "Invalid shape for elevated point: {}, must be a path",
                        key
                    ))),
                }
            })
            .collect()
    }
}
