use serde::Deserialize;

use crate::animation::{Animation, easing::Easing};

#[derive(Debug, Deserialize, Clone)]
pub struct Event {
    pub id: String,
    #[serde(default = "Vec::new")]
    pub depends: Vec<String>,
    pub object: String,
    pub easing: Easing,
    #[serde(flatten)]
    pub offset: Offset,
    #[serde(default = "default_start")]
    pub start: f32,
    pub duration: f32,
    #[serde(flatten)]
    pub animation: Animation,
}

fn default_start() -> f32 {
    0.0
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum Offset {
    /// 与被依赖动画同时开始，延迟 delay 秒
    #[serde(rename = "delay")]
    WithPrevious { delay: f32 },
    /// 被依赖动画完全结束后，延迟 offset 秒开始
    #[serde(rename = "offset")]
    AfterPrevious { offset: f32 },
    /// 检测被依赖的动画进行到的位置，与对象中心点距离最近时触发，延迟 offset 秒开始
    #[serde(rename = "track")]
    Track { offset: f32 },
}

impl Default for Offset {
    fn default() -> Self {
        Offset::AfterPrevious { offset: 0.0 }
    }
}
