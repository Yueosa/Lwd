use crate::config::world::{WorldConfig, WorldSize};
use crate::core::layer::{build_layers, LayerDefinition};
use crate::core::CoreError;

pub const AIR_BLOCK_ID: u8 = 1;

#[derive(Debug, Clone)]
pub struct WorldSizeSpec {
    pub key: String,
    pub width: u32,
    pub height: u32,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct WorldProfile {
    pub size: WorldSizeSpec,
    pub layers: Vec<LayerDefinition>,
}

#[derive(Debug, Clone)]
pub struct World {
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<u8>,
}

impl World {
    pub fn new_filled(width: u32, height: u32, block_id: u8) -> Self {
        let len = (width as usize) * (height as usize);
        Self {
            width,
            height,
            tiles: vec![block_id; len],
        }
    }

    pub fn new_air(width: u32, height: u32) -> Self {
        Self::new_filled(width, height, AIR_BLOCK_ID)
    }
}

impl WorldProfile {
    pub fn from_config(
        config: &WorldConfig,
        size_key: &str,
        custom_size: Option<(u32, u32)>,
    ) -> Result<Self, CoreError> {
        let size_cfg = config
            .world_sizes
            .get(size_key)
            .ok_or_else(|| CoreError::MissingWorldSize(size_key.to_string()))?;

        let (width, height) = resolve_size(size_key, size_cfg, custom_size)?;
        let size = WorldSizeSpec {
            key: size_key.to_string(),
            width,
            height,
            description: size_cfg.description.clone(),
        };

        let layers = build_layers(config)?;

        Ok(Self { size, layers })
    }

    pub fn create_world(&self) -> World {
        World::new_air(self.size.width, self.size.height)
    }
}

fn resolve_size(
    size_key: &str,
    size_cfg: &WorldSize,
    custom_size: Option<(u32, u32)>,
) -> Result<(u32, u32), CoreError> {
    if size_key == "custom" {
        if let Some((width, height)) = custom_size {
            if width > 0 && height > 0 {
                return Ok((width, height));
            }
        }
        return Err(CoreError::InvalidCustomSize);
    }

    match (size_cfg.width, size_cfg.height) {
        (Some(width), Some(height)) if width > 0 && height > 0 => Ok((width, height)),
        _ => Err(CoreError::InvalidCustomSize),
    }
}
