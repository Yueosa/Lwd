//! 海洋生成步骤

use crate::core::geometry::{self, Rect, Shape, ShapeParams, ShapeRecord};
use crate::generation::algorithm::RuntimeContext;

use super::BiomeDivisionAlgorithm;

pub fn execute(algo: &BiomeDivisionAlgorithm, ctx: &mut RuntimeContext) -> Result<(), String> {
    let ocean_id = algo.get_biome_id("ocean")
        .ok_or("未找到 ocean 环境定义")?;
    
    let bm = ctx.biome_map.as_mut().ok_or("需先执行太空/地狱填充")?;
    let w = bm.width as i32;
    let h = bm.height as i32;
    
    let y_top = (h as f64 * algo.params.ocean_top_limit) as i32;
    let y_bottom = (h as f64 * algo.params.ocean_bottom_limit) as i32;
    
    // 左侧海洋
    let left_width = (w as f64 * algo.params.ocean_left_width) as i32;
    let left_rect = Rect::new(0, y_top, left_width, y_bottom);
    geometry::fill_biome(&left_rect, bm, ocean_id);
    ctx.shape_log.push(ShapeRecord {
        label: "左侧海洋".into(),
        bbox: left_rect.bounding_box(),
        color: algo.biome_color(ocean_id),
        params: ShapeParams::from_rect(&left_rect),
    });
    
    // 右侧海洋
    let right_width = (w as f64 * algo.params.ocean_right_width) as i32;
    let right_rect = Rect::new(w - right_width, y_top, w, y_bottom);
    geometry::fill_biome(&right_rect, bm, ocean_id);
    ctx.shape_log.push(ShapeRecord {
        label: "右侧海洋".into(),
        bbox: right_rect.bounding_box(),
        color: algo.biome_color(ocean_id),
        params: ShapeParams::from_rect(&right_rect),
    });
    
    Ok(())
}
