use serde::Deserialize;

use crate::animation::easing::Easing;

#[derive(Debug, Deserialize)]
pub struct Event {
    pub id: String,
    #[serde(default = "Vec::new")]
    pub depends: Vec<String>,
    pub object: String,
    pub easing: Easing,
    pub offset: f64,
    pub duration: f64,
}
