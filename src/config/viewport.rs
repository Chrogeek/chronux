use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum TimeKey {
    /// 与被依赖动画同时开始，延迟 delay 秒
    #[serde(rename = "delay")]
    WithPrevious { depends: String, delay: f32 },
    /// 被依赖动画完全结束后，延迟 offset 秒开始
    #[serde(rename = "offset")]
    AfterPrevious { depends: String, offset: f32 },
}

#[derive(Debug, Deserialize, Clone)]
pub struct ViewportKeyframe {
    #[serde(flatten)]
    pub time: TimeKey,
    pub x: Option<f32>,
    pub y: Option<f32>,
    #[serde(rename = "width")]
    pub size: Option<f32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ViewportTrack {
    #[serde(rename = "tracked-path")]
    pub tracked_path: String,
    pub keyframes: Vec<ViewportKeyframe>,
}
