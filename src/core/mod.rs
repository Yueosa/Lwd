pub mod biome;
pub mod block;
pub mod color;
pub mod layer;
pub mod world;

use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum CoreError {
    MissingWorldSize(String),
    InvalidCustomSize,
    InvalidLayerPercent {
        name: String,
        start: u8,
        end: u8,
    },
}

impl Display for CoreError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingWorldSize(size_name) => {
                write!(f, "未找到世界尺寸配置: {size_name}")
            }
            Self::InvalidCustomSize => write!(f, "custom 世界尺寸需要传入有效宽高"),
            Self::InvalidLayerPercent { name, start, end } => write!(
                f,
                "层级百分比非法: {name} (start={start}, end={end})，要求 0<=start<end<=100"
            ),
        }
    }
}

impl Error for CoreError {}
