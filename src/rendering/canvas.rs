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

/// 降采样颜色图像：每 `factor×factor` 像素块取左上角颜色。
///
/// 生成进行中使用降采样纹理，减少 CPU→GPU 传输量。
/// `factor` 建议值：小世界 1，中世界 2，大世界 4。
pub fn world_to_color_image_downsampled(
    world: &World,
    lut: &[Color32; 256],
    factor: u32,
) -> ColorImage {
    if factor <= 1 {
        return world_to_color_image(world, lut);
    }
    let f = factor as usize;
    let w = world.width as usize;
    let h = world.height as usize;
    let out_w = (w + f - 1) / f;
    let out_h = (h + f - 1) / f;

    let mut pixels = vec![Color32::TRANSPARENT; out_w * out_h];

    pixels
        .par_chunks_mut(out_w)
        .enumerate()
        .for_each(|(out_y, row_pixels)| {
            let src_y = out_y * f;
            let row_start = src_y * w;
            for out_x in 0..out_w {
                let src_x = out_x * f;
                let idx = row_start + src_x;
                if idx < world.tiles.len() {
                    row_pixels[out_x] = lut[world.tiles[idx] as usize];
                }
            }
        });

    ColorImage {
        size: [out_w, out_h],
        pixels,
    }
}

/// 仅更新指定行范围 [y_start, y_end) 的颜色图像区域。
///
/// 返回 (y_start, 行像素数据) 用于局部纹理更新。
pub fn world_rows_to_color_pixels(
    world: &World,
    lut: &[Color32; 256],
    y_start: usize,
    y_end: usize,
) -> Vec<Color32> {
    let w = world.width as usize;
    let y_end = y_end.min(world.height as usize);
    let row_count = y_end.saturating_sub(y_start);
    let total = w * row_count;

    let mut pixels = vec![Color32::TRANSPARENT; total];

    pixels
        .par_chunks_mut(w)
        .enumerate()
        .for_each(|(ri, row_pixels)| {
            let y = y_start + ri;
            let row_start = y * w;
            for x in 0..w {
                row_pixels[x] = lut[world.tiles[row_start + x] as usize];
            }
        });

    pixels
}

/// 仅渲染世界的一个矩形子区域 [rx, ry, rw×rh] → ColorImage
///
/// 用于视口裁剪渲染——只生成当前可见区域+缓冲区的像素。
/// 越放大可见区域越小，需要渲染的像素越少，越流畅。
pub fn world_to_color_image_region(
    world: &World,
    lut: &[Color32; 256],
    rx: u32,
    ry: u32,
    rw: u32,
    rh: u32,
) -> ColorImage {
    let rw = rw as usize;
    let rh = rh as usize;
    let rx = rx as usize;
    let ry = ry as usize;
    let ww = world.width as usize;

    let mut pixels = vec![Color32::TRANSPARENT; rw * rh];

    pixels
        .par_chunks_mut(rw)
        .enumerate()
        .for_each(|(row_i, row_pixels)| {
            let src_y = ry + row_i;
            let src_row_start = src_y * ww + rx;
            for x in 0..rw {
                row_pixels[x] = lut[world.tiles[src_row_start + x] as usize];
            }
        });

    ColorImage {
        size: [rw, rh],
        pixels,
    }
}

/// 渲染世界子区域 [rx, ry, rw×rh] 并按 `lod` 倍降采样。
///
/// 输出尺寸为 `(rw/lod, rh/lod)`。每个输出像素取源 NxN 块的左上角颜色。
/// 当 zoom < 1 时，屏幕上每个像素覆盖多个世界像素，全分辨率是浪费。
/// LOD 让纹理像素数与屏幕像素数匹配，zoom 越小越快。
pub fn world_to_color_image_region_lod(
    world: &World,
    lut: &[Color32; 256],
    rx: u32,
    ry: u32,
    rw: u32,
    rh: u32,
    lod: u32,
) -> ColorImage {
    if lod <= 1 {
        return world_to_color_image_region(world, lut, rx, ry, rw, rh);
    }

    let f = lod as usize;
    let rw = rw as usize;
    let rh = rh as usize;
    let rx = rx as usize;
    let ry = ry as usize;
    let ww = world.width as usize;

    let out_w = (rw + f - 1) / f;
    let out_h = (rh + f - 1) / f;

    let mut pixels = vec![Color32::TRANSPARENT; out_w * out_h];

    pixels
        .par_chunks_mut(out_w)
        .enumerate()
        .for_each(|(out_row, row_pixels)| {
            let src_y = ry + out_row * f;
            let src_row_start = src_y * ww;
            for out_x in 0..out_w {
                let src_x = rx + out_x * f;
                let idx = src_row_start + src_x;
                if idx < world.tiles.len() {
                    row_pixels[out_x] = lut[world.tiles[idx] as usize];
                }
            }
        });

    ColorImage {
        size: [out_w, out_h],
        pixels,
    }
}
