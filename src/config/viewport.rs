use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct TimeKey {
    pub depends: String,
    pub offset: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ViewportKeyframe {
    #[serde(flatten)]
    pub time: TimeKey,
    pub x: f32,
    pub y: f32,
    pub size: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ViewportTrack {
    #[serde(rename = "tracked-path")]
    pub tracked_path: String,
    pub keyframes: Vec<ViewportKeyframe>,
}
