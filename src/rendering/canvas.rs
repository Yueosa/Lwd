use std::collections::HashMap;

use egui::{Color32, ColorImage};

use crate::core::block::BlockDefinition;
use crate::core::world::World;

pub fn build_color_map(blocks: &[BlockDefinition]) -> HashMap<u8, Color32> {
    let mut map = HashMap::with_capacity(blocks.len());

    for block in blocks {
        let [r, g, b, a] = block.color.as_array();
        map.insert(block.id, Color32::from_rgba_unmultiplied(r, g, b, a));
    }

    map
}

pub fn world_to_color_image(world: &World, colors: &HashMap<u8, Color32>) -> ColorImage {
    let mut pixels = Vec::with_capacity((world.width * world.height) as usize);

    for &tile in &world.tiles {
        let color = colors
            .get(&tile)
            .copied()
            .unwrap_or(Color32::from_rgba_unmultiplied(255, 0, 255, 255));
        pixels.push(color);
    }

    ColorImage {
        size: [world.width as usize, world.height as usize],
        pixels,
    }
}
