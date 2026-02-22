//! 沙漠生成步骤

use crate::core::biome::{BiomeMap, BIOME_UNASSIGNED};
use crate::core::geometry::{self, Ellipse, Rect, Shape, ShapeParams, ShapeRecord};
use crate::generation::algorithm::RuntimeContext;
use rand::Rng;

use super::BiomeDivisionAlgorithm;

pub fn execute(algo: &BiomeDivisionAlgorithm, ctx: &mut RuntimeContext) -> Result<(), String> {
    let desert_surface_id = algo.get_biome_id("desert")
        .ok_or("未找到 desert 环境定义")?;
    let desert_true_id = algo.get_biome_id("desert_true")
        .ok_or("未找到 desert_true 环境定义")?;
    
    let bm = ctx.biome_map.as_mut().ok_or("需先执行海洋生成")?;
    let w = bm.width as i32;
    let h = bm.height as i32;
    
    // ── 阶段 1：预计算常量 ─────────────────────────────
    let surface_top_y = (h as f64 * algo.params.desert_surface_top_limit) as i32;
    let surface_bottom_y = (h as f64 * algo.params.desert_surface_bottom_limit) as i32;
    let world_center_x = w / 2;
    
    // 真沙漠椭圆数学参数
    let true_top = h as f64 * algo.params.desert_true_top_limit;
    let true_bottom = h as f64 * algo.params.desert_true_bottom_limit
        * algo.params.desert_true_depth_factor;
    let ell_cy = (true_top + true_bottom) / 2.0;
    let ell_ry = (true_bottom - true_top) / 2.0;
    let junction_y = h as f64 * algo.params.desert_surface_bottom_limit;
    
    // 扫描地表层中间高度的空白区段
    let scan_y = ((surface_top_y + surface_bottom_y) / 2) as u32;
    let mut empty_ranges: Vec<(i32, i32)> = Vec::new();
    {
        let mut range_start: Option<i32> = None;
        for x in 0..w {
            if bm.get(x as u32, scan_y) == BIOME_UNASSIGNED {
                if range_start.is_none() {
                    range_start = Some(x);
                }
            } else if let Some(start) = range_start {
                empty_ranges.push((start, x));
                range_start = None;
            }
        }
        if let Some(start) = range_start {
            empty_ranges.push((start, w));
        }
    }
    
    // 辅助：验证矩形区域全空白（采样步长 2）—— 使用 geometry API
    let rect_all_empty = |bm: &BiomeMap, xl: i32, xr: i32, yt: i32, yb: i32| -> bool {
        geometry::shape_all_match(
            &Rect::new(xl, yt, xr, yb),
            bm, 2,
            |c| c == BIOME_UNASSIGNED,
        )
    };
    
    // 辅助：验证椭圆区域（从 true_top 到 true_bottom）全空白 —— 使用 geometry API
    let ellipse_all_empty = |bm: &BiomeMap, cx: i32, rx: f64| -> bool {
        if ell_ry <= 0.0 { return true; }
        geometry::shape_all_match(
            &Ellipse::new(cx as f64, ell_cy, rx, ell_ry),
            bm, 2,
            |c| c == BIOME_UNASSIGNED,
        )
    };
    
    // 辅助：计算椭圆 rx
    let compute_rx = |surface_half_width: f64| -> Option<f64> {
        if ell_ry <= 0.0 { return None; }
        let dy = (junction_y - ell_cy) / ell_ry;
        let dy_sq = dy * dy;
        if dy_sq >= 1.0 { return None; }
        Some(surface_half_width / (1.0 - dy_sq).sqrt())
    };
    
    // 沙漠槽位数据结构
    struct DesertSlot {
        center_x: i32,
        width: i32,
        has_true: bool,
        rx: f64,
    }
    let mut slots: Vec<DesertSlot> = Vec::new();
    
    let surface_count = algo.params.desert_surface_count as usize;
    let true_count = algo.params.desert_true_count as usize;
    let min_spacing = (w as f64 * algo.params.desert_surface_min_spacing) as i32;
    
    let spacing_ok = |slots: &[DesertSlot], cx: i32, width: i32, min_sp: i32| -> bool {
        for slot in slots {
            let dist = (cx - slot.center_x).abs();
            let required = (width + slot.width) / 2 + min_sp;
            if dist < required {
                return false;
            }
        }
        true
    };
    
    // ── 阶段 2：优先放置真沙漠 ────────────────────────
    if true_count > 0 && ell_ry > 0.0 {
        let mut ranges_by_center: Vec<(i32, i32)> = empty_ranges.clone();
        ranges_by_center.sort_by_key(|&(s, e)| {
            let mid = (s + e) / 2;
            (mid - world_center_x).abs()
        });
        
        let mut true_placed = 0;
        
        for &(range_start, range_end) in &ranges_by_center {
            if true_placed >= true_count { break; }
            
            let avg_width_ratio = (algo.params.desert_surface_width_min
                + algo.params.desert_surface_width_max) / 2.0;
            let width = (w as f64 * avg_width_ratio) as i32;
            let half_width = width / 2;
            
            if range_end - range_start < width { continue; }
            
            let min_cx = range_start + half_width;
            let max_cx = range_end - half_width;
            if min_cx >= max_cx { continue; }
            
            // 在区段内按离中心距离升序尝试多个位置
            let closest_to_center = world_center_x.clamp(min_cx, max_cx);
            let scan_step = (width / 2).max(4);
            
            let mut try_positions: Vec<i32> = Vec::new();
            try_positions.push(closest_to_center);
            let mut offset = scan_step;
            while closest_to_center - offset >= min_cx
                || closest_to_center + offset <= max_cx
            {
                if closest_to_center - offset >= min_cx {
                    try_positions.push(closest_to_center - offset);
                }
                if closest_to_center + offset <= max_cx {
                    try_positions.push(closest_to_center + offset);
                }
                offset += scan_step;
            }
            
            for cx in try_positions {
                if true_placed >= true_count { break; }
                
                if !spacing_ok(&slots, cx, width, min_spacing) { continue; }
                
                let xl = (cx - half_width).max(0);
                let xr = (cx + half_width).min(w);
                if !rect_all_empty(bm, xl, xr, surface_top_y, surface_bottom_y.min(h)) {
                    continue;
                }
                
                let surface_half_width = width as f64 / 2.0;
                if let Some(rx) = compute_rx(surface_half_width) {
                    if ellipse_all_empty(bm, cx, rx) {
                        slots.push(DesertSlot {
                            center_x: cx,
                            width,
                            has_true: true,
                            rx,
                        });
                        true_placed += 1;
                    }
                }
            }
        }
    }
    
    // ── 阶段 3：放置剩余普通地表沙漠 ──────────────────
    let remaining = surface_count.saturating_sub(slots.len());
    let mut attempts = 0u32;
    let max_attempts = (remaining as u32 + 1) * 30;
    let mut surface_placed = 0;
    
    while surface_placed < remaining && attempts < max_attempts {
        attempts += 1;
        
        let width_ratio = ctx.rng.gen_range(
            algo.params.desert_surface_width_min..=algo.params.desert_surface_width_max
        );
        let width = (w as f64 * width_ratio) as i32;
        let half_width = width / 2;
        
        let valid_ranges: Vec<_> = empty_ranges.iter()
            .filter(|&&(s, e)| e - s >= width)
            .collect();
        if valid_ranges.is_empty() { break; }
        
        let range_idx = ctx.rng.gen_range(0..valid_ranges.len());
        let &(rs, re) = valid_ranges[range_idx];
        
        let min_cx = rs + half_width;
        let max_cx = re - half_width;
        if min_cx >= max_cx { continue; }
        let cx = ctx.rng.gen_range(min_cx..max_cx);
        
        if !spacing_ok(&slots, cx, width, min_spacing) { continue; }
        
        let xl = (cx - half_width).max(0);
        let xr = (cx + half_width).min(w);
        if !rect_all_empty(bm, xl, xr, surface_top_y, surface_bottom_y.min(h)) {
            continue;
        }
        
        slots.push(DesertSlot {
            center_x: cx,
            width,
            has_true: false,
            rx: 0.0,
        });
        surface_placed += 1;
    }
    
    // ── 阶段 4：一次性绘制 + 保存槽位信息 ──────────────
    let mut slot_data: Vec<(i32, i32)> = Vec::new();
    let mut true_slot_data: Vec<(i32, i32)> = Vec::new();
    
    for slot in &slots {
        let half_width = slot.width / 2;
        let xl = (slot.center_x - half_width).max(0);
        let xr = (slot.center_x + half_width).min(w);
        
        // 绘制地表沙漠矩形 —— geometry API
        let surface_rect = Rect::new(xl, surface_top_y, xr, surface_bottom_y.min(h));
        geometry::fill_biome_if(&surface_rect, bm, desert_surface_id, |c| c == BIOME_UNASSIGNED);
        ctx.shape_log.push(ShapeRecord {
            label: format!("地表沙漠 #{}", slot_data.len() + 1),
            bbox: surface_rect.bounding_box(),
            color: algo.biome_color(desert_surface_id),
            params: ShapeParams::from_rect(&surface_rect),
        });
        
        // 绘制真沙漠完整椭圆（覆写其内部的地表沙漠）—— geometry API
        if slot.has_true {
            let ell = Ellipse::new(slot.center_x as f64, ell_cy, slot.rx, ell_ry);
            geometry::fill_biome_if(&ell, bm, desert_true_id, |c| {
                c == BIOME_UNASSIGNED || c == desert_surface_id
            });
            ctx.shape_log.push(ShapeRecord {
                label: "真沙漠椭圆".into(),
                bbox: ell.bounding_box(),
                color: algo.biome_color(desert_true_id),
                params: ShapeParams::from_ellipse(&ell),
            });
            true_slot_data.push((slot.center_x, slot.width));
        }
        
        slot_data.push((slot.center_x, slot.width));
    }
    
    // 保存槽位信息到 shared，供森林填充步骤判断真沙漠候选
    ctx.shared.insert("desert_slots".into(), Box::new(slot_data));
    ctx.shared.insert("desert_true_slots".into(), Box::new(true_slot_data));
    
    Ok(())
}
