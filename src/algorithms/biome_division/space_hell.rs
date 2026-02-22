//! 太空/地狱填充步骤

use crate::core::biome::{BiomeMap, BIOME_UNASSIGNED};
use crate::core::geometry::{self, Rect, Shape, ShapeParams, ShapeRecord};
use crate::generation::algorithm::RuntimeContext;

use super::BiomeDivisionAlgorithm;

pub fn execute(algo: &BiomeDivisionAlgorithm, ctx: &mut RuntimeContext) -> Result<(), String> {
    let space_id = algo.get_biome_id("space")
        .ok_or("未找到 space 环境定义")?;
    let hell_id = algo.get_biome_id("hell")
        .ok_or("未找到 hell 环境定义")?;
    
    let w = ctx.world.width as i32;
    let h = ctx.world.height as i32;
    
    // 读取层级边界（在取可变借用之前）
    let space_bottom = ctx.layer_end_px("space").ok_or("未找到 space 层级定义")? as i32;
    let hell_top = ctx.layer_start_px("hell").ok_or("未找到 hell 层级定义")? as i32;
    
    // 初始化 BiomeMap（全部填充为 UNASSIGNED）
    *ctx.biome_map = Some(BiomeMap::new_filled(w as u32, h as u32, BIOME_UNASSIGNED));
    let bm = ctx.biome_map.as_mut().unwrap();
    
    // 太空层
    let space_rect = Rect::new(0, 0, w, space_bottom);
    geometry::fill_biome(&space_rect, bm, space_id);
    ctx.shape_log.push(ShapeRecord {
        label: "太空层".into(),
        bbox: space_rect.bounding_box(),
        color: algo.biome_color(space_id),
        params: ShapeParams::from_rect(&space_rect),
    });
    
    // 地狱层
    let hell_rect = Rect::new(0, hell_top, w, h);
    geometry::fill_biome(&hell_rect, bm, hell_id);
    ctx.shape_log.push(ShapeRecord {
        label: "地狱层".into(),
        bbox: hell_rect.bounding_box(),
        color: algo.biome_color(hell_id),
        params: ShapeParams::from_rect(&hell_rect),
    });
    
    Ok(())
}
