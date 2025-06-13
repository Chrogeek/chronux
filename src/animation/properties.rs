use crate::animation::Animation;

#[derive(Debug, Clone)]
pub struct Properties;

impl Properties {
    pub fn apply(&self, event: &Animation, t: f64) -> Properties {
        self.clone()
    }
}
