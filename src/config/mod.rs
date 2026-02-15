pub mod blocks;
pub mod world;

use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum ConfigError {
    Parse(serde_json::Error),
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(error) => write!(f, "配置解析失败: {error}"),
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Parse(error) => Some(error),
        }
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(value: serde_json::Error) -> Self {
        Self::Parse(value)
    }
}
