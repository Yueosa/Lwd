use crate::config::world::WorldConfig;
use crate::core::CoreError;

#[derive(Debug, Clone)]
pub struct LayerDefinition {
    pub key: String,
    pub start_percent: u8,
    pub end_percent: u8,
    /// 短名称（用于 UI 标签，如 "太空"、"地表"），从 world.json 读取
    pub short_name: String,
    pub description: String,
}

impl LayerDefinition {
    pub fn bounds_for_height(&self, height: u32) -> (u32, u32) {
        let start = height * self.start_percent as u32 / 100;
        let end = height * self.end_percent as u32 / 100;
        (start, end)
    }
}

pub fn build_layers(config: &WorldConfig) -> Result<Vec<LayerDefinition>, CoreError> {
    let mut layers = Vec::with_capacity(config.layers.len());

    for (key, layer) in &config.layers {
        if !(layer.start_percent < layer.end_percent && layer.end_percent <= 100) {
            return Err(CoreError::InvalidLayerPercent {
                name: key.clone(),
                start: layer.start_percent,
                end: layer.end_percent,
            });
        }

        layers.push(LayerDefinition {
            key: key.clone(),
            start_percent: layer.start_percent,
            end_percent: layer.end_percent,
            short_name: layer.short_name.clone().unwrap_or_else(|| key.clone()),
            description: layer.description.clone(),
        });
    }

    Ok(layers)
}
