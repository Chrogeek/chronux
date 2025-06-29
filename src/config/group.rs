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
    #[serde(default)]
    #[serde(rename = "marked")]
    pub timeline_marked: bool,
}

impl Group {
    pub fn to_events(self) -> Vec<Event> {
        self.animations
            .into_iter()
            .enumerate()
            .map(|(i, (obj_id, animation))| {
                let track_follows = matches!(&self.offset, Offset::Track { .. }) && i != 0;
                Event {
                    id: format!(
                        "{}{}",
                        self.id,
                        if i == 0 { String::new() } else { i.to_string() }
                    ),
                    depends: if track_follows {
                        vec![self.id.clone()]
                    } else {
                        self.depends.clone()
                    },
                    object: obj_id.clone(),
                    easing: self.easing.clone(),
                    offset: {
                        if track_follows {
                            Offset::WithPrevious { delay: 0.0 }
                        } else {
                            self.offset.clone()
                        }
                    },
                    start: f32::NAN, // Start time will be calculated later
                    duration: self.duration,
                    animation: animation.clone(),
                    timeline_marked: self.timeline_marked && i == 0, // Only the first animation is marked
                }
            })
            .collect()
    }
}
