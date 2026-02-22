//! 丛林生成步骤

use crate::core::biome::BIOME_UNASSIGNED;
use crate::core::geometry::{self, Ellipse, Rect, Shape, ShapeCombine, ShapeParams, ShapeRecord};
use crate::generation::algorithm::RuntimeContext;
use rand::Rng;

use super::BiomeDivisionAlgorithm;

pub fn execute(algo: &BiomeDivisionAlgorithm, ctx: &mut RuntimeContext) -> Result<(), String> {
    let jungle_id = algo.get_biome_id("jungle")
        .ok_or("未找到 jungle 环境定义")?;
    
    let bm = ctx.biome_map.as_mut().ok_or("需先执行海洋生成")?;
    let w = bm.width as i32;
    let h = bm.height as i32;
    
    // 基于 RNG 随机选择左/右
    let place_on_left = ctx.rng.gen_bool(0.5);
    
    // 保存到 shared 供雪原生成使用
    ctx.shared.insert("jungle_on_left".into(), Box::new(place_on_left));
    
    // 计算森林边界（水平居中，半宽 = forest_width_ratio）
    let forest_center = w / 2;
    let forest_half_width = (w as f64 * algo.params.forest_width_ratio) as i32;
    let forest_left = forest_center - forest_half_width;
    let forest_right = forest_center + forest_half_width;
    
    // 计算海洋边界
    let ocean_left_right = (w as f64 * algo.params.ocean_left_width) as i32;
    let ocean_right_left = w - (w as f64 * algo.params.ocean_right_width) as i32;
    
    // 计算丛林可用空间和基础中心点
    let (jungle_cx_base, available_width) = if place_on_left {
        // 左侧：海洋右边界 → 森林左边界
        let left = ocean_left_right;
        let right = forest_left;
        let width = right - left;
        let center = left + width / 2;
        (center, width)
    } else {
        // 右侧：森林右边界 → 海洋左边界
        let left = forest_right;
        let right = ocean_right_left;
        let width = right - left;
        let center = left + width / 2;
        (center, width)
    };
    
    // 添加随机偏移（在可用宽度的 ±offset_range 范围内）
    let max_offset = (available_width as f64 * algo.params.jungle_center_offset_range) as i32;
    let offset = ctx.rng.gen_range(-max_offset..=max_offset);
    let jungle_cx = jungle_cx_base + offset;
    
    // 丛林椭圆参数
    let jungle_rx = (w as f64 * algo.params.jungle_width_ratio / 2.0) as i32;
    let jungle_cy = h / 2;  // 椭圆中心在世界垂直中心
    let jungle_ry = h / 2;  // 椭圆半径覆盖整个世界高度
    
    // 实际写入范围（裁剪）
    let top_y = (h as f64 * algo.params.jungle_top_limit) as i32;
    let bottom_y = (h as f64 * algo.params.jungle_bottom_limit) as i32;
    
    // 丛林椭圆 + y范围裁剪（椭圆 ∩ 矩形）
    let ell = Ellipse::new(
        jungle_cx as f64, jungle_cy as f64,
        jungle_rx as f64, jungle_ry as f64,
    );
    let ell_params = ShapeParams::from_ellipse(&ell);
    let clip = Rect::new(0, top_y, w, bottom_y);
    let shape = ell.intersect(clip);
    geometry::fill_biome_if(&shape, bm, jungle_id, |c| c == BIOME_UNASSIGNED);
    ctx.shape_log.push(ShapeRecord {
        label: "丛林".into(),
        bbox: shape.bounding_box(),
        color: algo.biome_color(jungle_id),
        params: ell_params,
    });
    
    Ok(())
}
