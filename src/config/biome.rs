use std::collections::BTreeMap;

use serde::Deserialize;

use crate::config::ConfigError;

const BIOME_JSON: &str = include_str!("../assets/biome.json");

#[derive(Debug, Clone, Deserialize)]
pub struct BiomeConfig {
    pub key: String,
    pub name: String,
    pub overlay_color: [u8; 4],
    pub generation_weight: u32,
    pub description: String,
}

pub type BiomesConfig = BTreeMap<u8, BiomeConfig>;

pub fn load_biomes_config() -> Result<BiomesConfig, ConfigError> {
    let config: BiomesConfig = serde_json::from_str(BIOME_JSON)?;
    Ok(config)
}
