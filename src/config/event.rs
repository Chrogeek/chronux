use serde::Deserialize;

use crate::animation::{easing::Easing, Animation};

#[derive(Debug, Deserialize, Clone)]
pub struct Event {
    pub id: String,
    #[serde(default = "Vec::new")]
    pub depends: Vec<String>,
    pub object: String,
    pub easing: Easing,
    pub offset: f32,
    #[serde(default = "default_start")]
    pub start: f32,
    pub duration: f32,
    #[serde(flatten)]
    pub animation: Animation,
}

fn default_start() -> f32 {
    0.0
}
