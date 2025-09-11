use serde::Deserialize;
use serde_with::{OneOrMany, formats::PreferOne, serde_as};

use crate::{
    animation::{TaggedAnimation, easing::Easing},
    config::{
        counter::{Counters, merge_counters},
        event::{Event, Offset},
    },
};

#[serde_as]
#[derive(Debug, Deserialize, Clone)]
pub struct Group {
    pub id: String,
    #[serde(default = "Vec::new")]
    #[serde_as(as = "OneOrMany<_, PreferOne>")]
    pub depends: Vec<String>,
    #[serde_as(as = "OneOrMany<_, PreferOne>")]
    pub anilist: Vec<TaggedAnimation>,
    #[serde(flatten)]
    pub offset: Offset,
    pub easing: Easing,
    pub duration: f32,
    #[serde(default)]
    pub counters: Counters,
}

impl Group {
    pub fn to_events(&self) -> Vec<Event> {
        self.anilist
            .iter()
            .enumerate()
            .map(
                |(
                    i,
                    TaggedAnimation {
                        obj_id,
                        animation,
                        counters,
                        easing,
                    },
                )| {
                    let track_follows = matches!(&self.offset, Offset::Track { .. }) && i != 0;
                    Event {
                        id: if i == 0 {
                            self.id.clone()
                        } else {
                            format!("{}____{}", self.id, i)
                        },
                        depends: if track_follows {
                            vec![self.id.clone()]
                        } else {
                            self.depends.clone()
                        },
                        object: obj_id.clone(),
                        easing: easing.clone().unwrap_or(self.easing.clone()),
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
                            merge_counters(&self.counters, counters)
                        } else {
                            counters.clone()
                        },
                    }
                },
            )
            .collect()
    }
}
