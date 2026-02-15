use crate::config::blocks::BlocksConfig;
use crate::core::color::ColorRgba;

pub type BlockId = u8;

#[derive(Debug, Clone)]
pub struct BlockDefinition {
    pub id: BlockId,
    pub name: String,
    pub color: ColorRgba,
    pub description: String,
    pub category: String,
}

pub fn build_block_definitions(config: &BlocksConfig) -> Vec<BlockDefinition> {
    config
        .iter()
        .map(|(id, block)| BlockDefinition {
            id: *id,
            name: block.name.clone(),
            color: block.rgba.into(),
            description: block.description.clone(),
            category: block.category.clone(),
        })
        .collect()
}
