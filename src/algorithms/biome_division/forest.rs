//! 森林生成步骤

use crate::core::biome::BIOME_UNASSIGNED;
use crate::core::geometry::{self, Rect, Shape, ShapeParams, ShapeRecord};
use crate::generation::algorithm::RuntimeContext;

use super::BiomeDivisionAlgorithm;

pub fn execute(algo: &BiomeDivisionAlgorithm, ctx: &mut RuntimeContext) -> Result<(), String> {
    let forest_id = algo.get_biome_id("forest")
        .ok_or("未找到 forest 环境定义")?;
    
    let bm = ctx.biome_map.as_mut().ok_or("需先执行海洋生成")?;
    let w = bm.width as i32;
    let h = bm.height as i32;
    
    let y_top = (h as f64 * 0.10) as i32;
    let y_bottom = (h as f64 * 0.40) as i32;
    let center_x = w / 2;
    let half_width = (w as f64 * algo.params.forest_width_ratio) as i32;
    
    let shape = Rect::new(
        center_x - half_width, y_top,
        center_x + half_width, y_bottom,
    );
    geometry::fill_biome_if(&shape, bm, forest_id, |c| c == BIOME_UNASSIGNED);
    ctx.shape_log.push(ShapeRecord {
        label: "中心森林".into(),
        bbox: shape.bounding_box(),
        color: algo.biome_color(forest_id),
        params: ShapeParams::from_rect(&shape),
    });
    
    Ok(())
}
