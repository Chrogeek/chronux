use std::fmt::Display;

#[derive(Debug)]
pub enum ChronuxError {
    IoError(std::io::Error),
    ConfigError(toml::de::Error),
    XmlError(roxmltree::Error),
    CircularDependency(String),
    AttributeNotFound {
        node_id: Option<String>,
        attribute: String,
    },
    ParseFloatError(std::num::ParseFloatError),
}

pub type ChronuxResult<T> = Result<T, ChronuxError>;

impl Display for ChronuxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChronuxError::IoError(e) => write!(f, "IO Error: {}", e),
            ChronuxError::ConfigError(e) => write!(f, "Config Error: {}", e),
            ChronuxError::XmlError(e) => write!(f, "SVG Parse Error: {}", e),
            ChronuxError::CircularDependency(id) => {
                write!(f, "Circular dependency detected: {}", id)
            }
            ChronuxError::AttributeNotFound { node_id, attribute } => {
                if let Some(node_id) = node_id {
                    write!(f, "Attribute '{}' not found in node '{}'", attribute, node_id)
                } else {
                    write!(f, "Attribute '{}' not found in node", attribute)
                }
            }
            ChronuxError::ParseFloatError(e) => write!(f, "Floating-point parse error: {}", e),
        }
    }
}
