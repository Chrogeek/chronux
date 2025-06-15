use serde::Deserialize;

use crate::config::event::Offset;

#[derive(Debug)]
pub struct Viewport {
    pub depends: Vec<String>,
    pub offset: Offset,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl<'de> Deserialize<'de> for Viewport {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (x, y, width, depends, offset) = Deserialize::deserialize(deserializer)?;
        Ok(Viewport {
            x,
            y,
            width,
            height: f32::NAN,
            depends,
            offset,
        })
    }
}
