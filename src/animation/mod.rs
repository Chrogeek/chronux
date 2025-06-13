use serde::Deserialize;

pub mod easing;
pub mod properties;
pub mod timeline;

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "animation")]
pub enum Animation {
    #[serde(rename = "trim-path")]
    TrimPath { reverse: bool },
}
