use std::collections::BTreeMap;

use serde::Deserialize;

use crate::config::ConfigError;

const BLOCKS_JSON: &str = include_str!("../assets/blocks.json");

#[derive(Debug, Clone, Deserialize)]
pub struct BlockConfig {
    pub name: String,
    pub rgba: [u8; 4],
    pub description: String,
    pub category: String,
}

pub type BlocksConfig = BTreeMap<u8, BlockConfig>;

pub fn load_blocks_config() -> Result<BlocksConfig, ConfigError> {
    let config: BlocksConfig = serde_json::from_str(BLOCKS_JSON)?;
    Ok(config)
}
