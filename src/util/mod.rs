#[derive(Debug)]
pub enum ChronuxError {
    IoError(std::io::Error),
    ConfigError(toml::de::Error),
}

pub type ChronuxResult<T> = Result<T, ChronuxError>;
