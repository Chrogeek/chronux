use serde::Deserialize;

use crate::{config::event::Event, util::{ChronuxError, ChronuxResult}};

pub mod event;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub input: InputConfig,
    pub output: OutputConfig,
    pub events: Vec<Event>,
}

#[derive(Debug, Deserialize)]
pub struct InputConfig {
    pub svg: String,
    pub map: String,
}

#[derive(Debug, Deserialize)]
pub struct OutputConfig {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub duration: f64,
}

pub fn load(filename: &str) -> ChronuxResult<Config> {
    let config_data = std::fs::read_to_string(filename).map_err(|e| ChronuxError::IoError(e))?;
    toml::from_str(&config_data).map_err(|e| ChronuxError::ConfigError(e))
}
