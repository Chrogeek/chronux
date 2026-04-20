use serde::Deserialize;

use crate::config::viewport::TimeKey;

#[derive(Debug, Clone, Deserialize)]
pub struct EarthquakeEntry {
    #[serde(flatten)]
    pub time: TimeKey, // 触发时间，支持 delay 和 offset 两种模式
    pub duration: f32, // 持续时间，单位秒
    pub max_amp: f32,  // 最大振幅，单位像素
}

#[derive(Debug, Clone)]
pub struct ParsedEarthquake {
    pub start: f32,
    pub duration: f32,
    pub max_amp: f32,
}
