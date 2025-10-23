use core::f32;

use serde::Deserialize;
use serde_with::{OneOrMany, formats::PreferOne, serde_as};

use crate::{
    animation::{Animation, TaggedAnimation, easing::Easing},
    config::{
        counter::Counters,
        event::{Event, Offset},
        group::Group,
    },
};

#[serde_as]
#[derive(Debug, Deserialize, Clone)]
pub struct Tracker {
    pub id: String,
    #[serde(default = "Vec::new")]
    #[serde_as(as = "OneOrMany<_, PreferOne>")]
    pub depends: Vec<String>,
    #[serde(flatten)]
    pub offset: Offset,
    pub easing: Easing,
    pub duration: f32,
    pub object: String,
    #[serde(default)]
    pub counters: Counters,
    pub children: Vec<TrackerChild>,
    #[serde(flatten)]
    pub animation: Animation,
}

#[serde_as]
#[derive(Debug, Deserialize, Clone)]
pub struct TrackerChild {
    pub id: String,
    #[serde(flatten)]
    pub offset: Offset,
    pub easing: Easing,
    pub duration: f32,
    #[serde(default)]
    pub counters: Counters,
    #[serde_as(as = "OneOrMany<_, PreferOne>")]
    pub anilist: Vec<TaggedAnimation>,
}

impl Tracker {
    pub fn to_events(&self) -> Vec<Event> {
        self.children.iter().fold(
            vec![Event {
                id: self.id.clone(),
                depends: self.depends.clone(),
                object: self.object.clone(),
                easing: self.easing.clone(),
                offset: self.offset.clone(),
                start: f32::NAN,
                duration: self.duration,
                animation: self.animation.clone(),
                counters: self.counters.clone(),
            }],
            |mut acc, child| {
                let group = Group {
                    id: child.id.clone(),
                    depends: vec![self.id.clone()],
                    anilist: child.anilist.clone(),
                    offset: child.offset.clone(),
                    easing: child.easing.clone(),
                    duration: child.duration,
                    counters: child.counters.clone(),
                };
                acc.append(&mut group.to_events());
                acc
            },
        )
    }
}
