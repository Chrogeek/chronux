// use serde::Deserialize;

pub mod event;

// #[derive(Debug, Deserialize)]
// pub struct Config {
//     pub input: InputConfig,
//     pub output: OutputConfig,
//     // pub events: Vec<AnimationEvent>,
// }

// #[derive(Debug, Deserialize)]
// pub struct InputConfig {
//     pub svg: String,
// }

// #[derive(Debug, Deserialize)]
// pub struct OutputConfig {
//     pub width: u32,
//     pub height: u32,
//     pub fps: u32,
//     pub duration: f64,
// }

// #[derive(Debug, Deserialize)]
// #[serde(tag = "type")]
// pub enum AnimationEvent {
//     #[serde(rename = "camera")]
//     Camera {
//         id: String,
//         duration: f64,
//         #[serde(default)]
//         start: Option<f64>,
//         #[serde(flatten)]
//         curve: AnimationCurve,
//         params: CameraParams,
//         #[serde(default)]
//         trigger: Option<EventTrigger>,
//     },
//     // 其他事件类型...
// }

// #[derive(Debug, Deserialize)]
// #[serde(untagged)]
// pub enum AnimationCurve {
//     Bezier { bezier: [f64; 4] },
//     Preset { preset: String },
// }

// #[derive(Debug, Deserialize)]
// pub struct EventTrigger {
//     pub event: String,
//     #[serde(default)]
//     pub offset: f64,
//     // #[serde(default = "default_trigger_mode")]
//     pub mode: TriggerMode,
// }

// #[derive(Debug, Deserialize)]
// #[serde(rename_all = "snake_case")]
// pub enum TriggerMode {
//     AfterEnd,
//     WithStart,
// }

// pub fn load(filename: &str) -> Config {
//     let config_data = std::fs::read_to_string(filename).expect("Failed to read config file");
//     serde_json::from_str(&config_data).expect("Failed to parse config file")
// }
