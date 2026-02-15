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

/// Pre-build a 256-entry lookup table for O(1) block→color.
pub fn build_color_lut(colors: &HashMap<u8, Color32>) -> [Color32; 256] {
    let fallback = Color32::from_rgba_unmultiplied(255, 0, 255, 255);
    let mut lut = [fallback; 256];
    for (&id, &color) in colors {
        lut[id as usize] = color;
    }
    lut
}

pub fn world_to_color_image(world: &World, lut: &[Color32; 256]) -> ColorImage {
    let pixels: Vec<Color32> = world.tiles.iter().map(|&t| lut[t as usize]).collect();
    ColorImage {
        size: [world.width as usize, world.height as usize],
        pixels,
    }
}
