use serde::Deserialize;

pub mod easing;
pub mod timeline;

#[derive(Debug, Deserialize)]
pub enum Animation {
    TrimPath { reverse: bool },
}
