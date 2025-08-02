use std::collections::HashMap;

use serde::Deserialize;

use crate::{
    animation::{Animation, TaggedAnimation, easing::Easing},
    config::event::{Event, Offset},
};

#[derive(Debug, Deserialize, Clone)]
pub struct Group {
    pub id: String,
    #[serde(default = "Vec::new")]
    pub depends: Vec<String>,
    pub animations: HashMap<String, Animation>,
    pub anilist: Vec<TaggedAnimation>,
    #[serde(flatten)]
    pub offset: Offset,
    pub easing: Easing,
    pub duration: f32,
}

impl Group {
    pub fn to_events(self) -> Vec<Event> {
        let mut events = vec![];

        if !self.animations.is_empty() {
            log::warn!("'animations' property has been deprecated, use 'anilist' instead");
        }

        events.extend(
            self.animations
                .into_iter()
                .enumerate()
                .map(|(i, (obj_id, animation))| {
                    let track_follows = matches!(&self.offset, Offset::Track { .. }) && i != 0;
                    Event {
                        id: if i == 0 {
                            self.id.clone()
                        } else {
                            format!("{}____{}", self.id, i.to_string())
                        },
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
                    }
                }),
        );

        events.extend(self.anilist.into_iter().enumerate().map(
            |(i, TaggedAnimation { obj_id, animation })| {
                let track_follows = matches!(&self.offset, Offset::Track { .. }) && i != 0;
                Event {
                    id: if i == 0 {
                        self.id.clone()
                    } else {
                        format!("{}____{}", self.id, i.to_string())
                    },
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
                }
            },
        ));

        events
    }
}
