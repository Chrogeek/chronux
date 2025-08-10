use serde::Deserialize;

use crate::{
    animation::{TaggedAnimation, easing::Easing},
    config::{
        counter::{Counters, merge_counters},
        event::{Event, Offset},
    },
};

#[derive(Debug, Deserialize, Clone)]
pub struct Group {
    pub id: String,
    #[serde(default = "Vec::new")]
    pub depends: Vec<String>,
    pub anilist: Vec<TaggedAnimation>,
    #[serde(flatten)]
    pub offset: Offset,
    pub easing: Easing,
    pub duration: f32,
    #[serde(default)]
    pub counters: Counters,
}

impl Group {
    pub fn to_events(self) -> Vec<Event> {
        self.anilist
            .into_iter()
            .enumerate()
            .map(
                |(
                    i,
                    TaggedAnimation {
                        obj_id,
                        animation,
                        counters,
                    },
                )| {
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
                        counters: if i == 0 {
                            merge_counters(&self.counters, &counters)
                        } else {
                            counters
                        },
                    }
                },
            )
            .collect()
    }
}
