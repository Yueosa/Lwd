use std::collections::HashMap;

use egui::{Color32, ColorImage};
use rayon::prelude::*;

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

/// 将世界方块数据转换为颜色图像（rayon 并行按行转换）
pub fn world_to_color_image(world: &World, lut: &[Color32; 256]) -> ColorImage {
    let w = world.width as usize;
    let h = world.height as usize;
    let total = w * h;

    let mut pixels = vec![Color32::TRANSPARENT; total];

    // 按行并行：每行独立做 LUT 查表
    pixels
        .par_chunks_mut(w)
        .enumerate()
        .for_each(|(y, row_pixels)| {
            let row_start = y * w;
            for x in 0..w {
                row_pixels[x] = lut[world.tiles[row_start + x] as usize];
            }
        });

    ColorImage {
        size: [w, h],
        pixels,
    }
}
