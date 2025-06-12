use serde::Deserialize;

pub mod easing;

#[derive(Debug, Deserialize)]
pub enum Animation {
    TrimPath { reverse: bool },
}
