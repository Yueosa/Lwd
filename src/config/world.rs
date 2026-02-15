use std::collections::BTreeMap;

use serde::Deserialize;

use crate::config::ConfigError;

const WORLD_JSON: &str = include_str!("../assets/world.json");

#[derive(Debug, Clone, Deserialize)]
pub struct WorldConfig {
    pub world_sizes: BTreeMap<String, WorldSize>,
    pub layers: BTreeMap<String, LayerConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorldSize {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayerConfig {
    pub start_percent: u8,
    pub end_percent: u8,
    pub description: String,
}

pub fn load_world_config() -> Result<WorldConfig, ConfigError> {
    let config: WorldConfig = serde_json::from_str(WORLD_JSON)?;
    Ok(config)
}
