//! 雪原生成步骤

use crate::core::biome::BIOME_UNASSIGNED;
use crate::core::geometry::{self, Shape, ShapeParams, ShapeRecord, Trapezoid};
use crate::generation::algorithm::RuntimeContext;
use rand::Rng;

use super::BiomeDivisionAlgorithm;

pub fn execute(algo: &BiomeDivisionAlgorithm, ctx: &mut RuntimeContext) -> Result<(), String> {
    let snow_id = algo.get_biome_id("snow")
        .ok_or("未找到 snow 环境定义")?;
    
    let bm = ctx.biome_map.as_mut().ok_or("需先执行海洋生成")?;
    let w = bm.width as i32;
    let h = bm.height as i32;
    
    // 从 shared 读取丛林位置，雪原在对侧
    let jungle_on_left = ctx.shared.get("jungle_on_left")
        .and_then(|v| v.downcast_ref::<bool>())
        .copied()
        .unwrap_or(false);
    let place_on_left = !jungle_on_left;
    
    // 计算森林边界
    let forest_center = w / 2;
    let forest_half_width = (w as f64 * algo.params.forest_width_ratio) as i32;
    let forest_left = forest_center - forest_half_width;
    let forest_right = forest_center + forest_half_width;
    
    // 计算海洋边界
    let ocean_left_right = (w as f64 * algo.params.ocean_left_width) as i32;
    let ocean_right_left = w - (w as f64 * algo.params.ocean_right_width) as i32;
    
    // 计算雪原可用空间和基础中心点
    let (snow_cx_base, available_width) = if place_on_left {
        let left = ocean_left_right;
        let right = forest_left;
        let width = right - left;
        let center = left + width / 2;
        (center, width)
    } else {
        let left = forest_right;
        let right = ocean_right_left;
        let width = right - left;
        let center = left + width / 2;
        (center, width)
    };
    
    // 添加随机偏移
    let max_offset = (available_width as f64 * algo.params.snow_center_offset_range) as i32;
    let offset = ctx.rng.gen_range(-max_offset..=max_offset);
    let snow_cx = snow_cx_base + offset;
    
    // 梯形参数（上窄下宽）
    let top_half_width = (w as f64 * algo.params.snow_top_width_ratio / 2.0) as i32;
    let bottom_half_width = (w as f64 * algo.params.snow_bottom_width_ratio / 2.0) as i32;
    
    let top_y = (h as f64 * algo.params.snow_top_limit) as i32;
    let bottom_y = (h as f64 * algo.params.snow_bottom_limit * algo.params.snow_bottom_depth_factor) as i32;
    
    // 梯形填充（只替换 BIOME_UNASSIGNED）
    let shape = Trapezoid::new(
        top_y, bottom_y.min(h),
        (snow_cx - top_half_width) as f64,
        (snow_cx + top_half_width) as f64,
        (snow_cx - bottom_half_width) as f64,
        (snow_cx + bottom_half_width) as f64,
    );
    geometry::fill_biome_if(&shape, bm, snow_id, |c| c == BIOME_UNASSIGNED);
    ctx.shape_log.push(ShapeRecord {
        label: "雪原".into(),
        bbox: shape.bounding_box(),
        color: algo.biome_color(snow_id),
        params: ShapeParams::from_trapezoid(&shape),
    });
    
    Ok(())
}
