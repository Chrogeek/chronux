use std::collections::HashMap;

use serde::Deserialize;

use crate::{
    animation::{Animation, easing::Easing},
    config::event::{Event, Offset},
};

#[derive(Debug, Deserialize, Clone)]
pub struct Group {
    pub id: String,
    #[serde(default = "Vec::new")]
    pub depends: Vec<String>,
    pub animations: HashMap<String, Animation>,
    #[serde(flatten)]
    pub offset: Offset,
    pub easing: Easing,
    pub duration: f32,
}

impl Group {
    pub fn to_events(self) -> Vec<Event> {
        self.animations
            .into_iter()
            .enumerate()
            .map(|(i, (obj_id, animation))| Event {
                id: format!("{}{}", self.id, if i == 0 { String::new() } else { i.to_string() }),
                depends: self.depends.clone(),
                object: obj_id.clone(),
                easing: self.easing.clone(),
                offset: self.offset.clone(),
                start: f32::NAN, // Start time will be calculated later
                duration: self.duration,
                animation: animation.clone(),
            })
            .collect()
    }
}
