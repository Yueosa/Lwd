//! 地块填充步骤

use crate::core::biome::BIOME_UNASSIGNED;
use crate::core::geometry::{self, Rect, Shape, ShapeParams, ShapeRecord};
use crate::generation::algorithm::RuntimeContext;

use super::BiomeDivisionAlgorithm;

pub fn execute(algo: &BiomeDivisionAlgorithm, ctx: &mut RuntimeContext) -> Result<(), String> {
    let stone_id = algo.get_biome_id("stone")
        .ok_or("未找到 stone 环境定义")?;
    
    let bm = ctx.biome_map.as_mut().ok_or("需先执行前置步骤")?;
    let w = bm.width as i32;
    let h = bm.height as i32;
    
    let world_rect = Rect::new(0, 0, w, h);
    geometry::fill_biome_if(&world_rect, bm, stone_id, |c| c == BIOME_UNASSIGNED);
    ctx.shape_log.push(ShapeRecord {
        label: "地块填充".into(),
        bbox: world_rect.bounding_box(),
        color: algo.biome_color(stone_id),
        params: ShapeParams::from_rect(&world_rect),
    });
    
    Ok(())
}
