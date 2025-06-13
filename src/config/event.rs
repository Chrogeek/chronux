use serde::Deserialize;

use crate::animation::{easing::Easing, Animation};

#[derive(Debug, Deserialize, Clone)]
pub struct Event {
    pub id: String,
    #[serde(default = "Vec::new")]
    pub depends: Vec<String>,
    pub object: String,
    pub easing: Easing,
    pub offset: f64,
    #[serde(default = "default_start")]
    pub start: f64,
    pub duration: f64,
    #[serde(flatten)]
    pub animation: Animation,
}

fn default_start() -> f64 {
    0.0
}
