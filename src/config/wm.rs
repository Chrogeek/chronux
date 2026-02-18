use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct WatermarkConfig {
    pub samples: usize,
    pub len: usize,
    #[serde(rename = "watermark-path")]
    pub img_path: String,
    #[serde(rename = "watermark-meta-filename")]
    pub wminfo_filename: String,
}
