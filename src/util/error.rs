use std::fmt::Display;

#[derive(Debug)]
pub enum ChronuxError {
    VersionError(String),
    IoError(std::io::Error),
    SerializationError(toml::ser::Error),
    ConfigError(toml::de::Error),
    XmlError(roxmltree::Error),
    CircularDependency(String),
    AttributeNotFound {
        node_id: Option<String>,
        attribute: String,
    },
    ParseFloatError(std::num::ParseFloatError),
    ColorParseError(String),
    SvgError(String),
    SkiaError(String),
    ArgError(String),
    HrefError(String),
    BrokenDependency(String),
    FfmpegError(String),
    ImageDecodeError(String),
}

pub type ChronuxResult<T> = Result<T, ChronuxError>;

impl From<toml::ser::Error> for ChronuxError {
    fn from(value: toml::ser::Error) -> Self {
        ChronuxError::SerializationError(value)
    }
}

impl From<toml::de::Error> for ChronuxError {
    fn from(value: toml::de::Error) -> Self {
        ChronuxError::ConfigError(value)
    }
}

impl From<std::io::Error> for ChronuxError {
    fn from(value: std::io::Error) -> Self {
        ChronuxError::IoError(value)
    }
}

impl From<roxmltree::Error> for ChronuxError {
    fn from(value: roxmltree::Error) -> Self {
        ChronuxError::XmlError(value)
    }
}

impl From<std::num::ParseFloatError> for ChronuxError {
    fn from(value: std::num::ParseFloatError) -> Self {
        ChronuxError::ParseFloatError(value)
    }
}

impl Display for ChronuxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChronuxError::VersionError(version) => {
                write!(f, "Incompatible version: {version}")
            }
            ChronuxError::IoError(e) => write!(f, "IO Error: {e}"),
            ChronuxError::SerializationError(e) => write!(f, "Serialization Error: {e}"),
            ChronuxError::ConfigError(e) => {
                write!(f, "Config Error at span '{:?}': {}", e.span(), e.message())
            }
            ChronuxError::XmlError(e) => write!(f, "SVG Parse Error: {e}"),
            ChronuxError::CircularDependency(id) => {
                write!(f, "Circular dependency detected: {id}")
            }
            ChronuxError::AttributeNotFound { node_id, attribute } => {
                if let Some(node_id) = node_id {
                    write!(f, "Attribute '{attribute}' not found in node '{node_id}'")
                } else {
                    write!(f, "Attribute '{attribute}' not found in node")
                }
            }
            ChronuxError::ParseFloatError(e) => write!(f, "Floating-point parse error: {e}"),
            ChronuxError::ColorParseError(color) => {
                write!(f, "Failed to parse color: '{color}'")
            }
            ChronuxError::SvgError(msg) => write!(f, "SVG Error: {msg}"),
            ChronuxError::SkiaError(msg) => write!(f, "Skia Error: {msg}"),
            ChronuxError::ArgError(msg) => write!(f, "Argument Error: {msg}"),
            ChronuxError::HrefError(msg) => write!(f, "Href Error: {msg}"),
            ChronuxError::BrokenDependency(id) => write!(f, "Broken dependency: {id}"),
            ChronuxError::FfmpegError(msg) => write!(f, "FFmpeg Error: {msg}"),
            ChronuxError::ImageDecodeError(msg) => write!(f, "Image Decode Error: {msg}"),
        }
    }
}
