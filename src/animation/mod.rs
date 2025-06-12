use serde::Deserialize;

pub mod curve;

#[derive(Debug, Deserialize)]
pub enum Animation {
    TrimPath { reverse: bool },
}
