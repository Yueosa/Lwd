//! 猩红生成步骤

use crate::core::biome::{BiomeMap, BIOME_UNASSIGNED};
use crate::core::geometry::{self, Rect, Shape, ShapeParams, ShapeRecord};
use crate::generation::algorithm::RuntimeContext;
use rand::Rng;

use super::BiomeDivisionAlgorithm;

pub fn execute(algo: &BiomeDivisionAlgorithm, ctx: &mut RuntimeContext) -> Result<(), String> {
    let crimson_id = algo.get_biome_id("crimson")
        .ok_or("未找到 crimson 环境定义")?;
    
    let bm = ctx.biome_map.as_mut().ok_or("需先执行海洋生成")?;
    let w = bm.width as i32;
    let h = bm.height as i32;
    
    let surface_top_y = (h as f64 * algo.params.crimson_top_limit) as i32;
    let surface_bottom_y = (h as f64 * algo.params.crimson_bottom_limit) as i32;
    let min_spacing = (w as f64 * algo.params.crimson_min_spacing) as i32;
    let count = algo.params.crimson_count as usize;
    
    if count == 0 { return Ok(()); }
    
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
    
    // 槽位记录
    struct CrimsonSlot {
        center_x: i32,
        width: i32,
    }
    let mut slots: Vec<CrimsonSlot> = Vec::new();
    
    // 间距检查
    let spacing_ok = |slots: &[CrimsonSlot], cx: i32, width: i32, min_sp: i32| -> bool {
        for slot in slots {
            let dist = (cx - slot.center_x).abs();
            let required = (width + slot.width) / 2 + min_sp;
            if dist < required {
                return false;
            }
        }
        true
    };
    
    // 随机放置猩红矩形
    let mut attempts = 0u32;
    let max_attempts = (count as u32 + 1) * 30;
    let mut placed = 0;
    
    while placed < count && attempts < max_attempts {
        attempts += 1;
        
        let width_ratio = ctx.rng.gen_range(
            algo.params.crimson_width_min..=algo.params.crimson_width_max
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
        
        slots.push(CrimsonSlot { center_x: cx, width });
        placed += 1;
    }
    
    // 一次性绘制 —— geometry API
    for (i, slot) in slots.iter().enumerate() {
        let half_width = slot.width / 2;
        let xl = (slot.center_x - half_width).max(0);
        let xr = (slot.center_x + half_width).min(w);
        
        let rect = Rect::new(xl, surface_top_y, xr, surface_bottom_y.min(h));
        geometry::fill_biome_if(&rect, bm, crimson_id, |c| c == BIOME_UNASSIGNED);
        ctx.shape_log.push(ShapeRecord {
            label: format!("猩红 #{}", i + 1),
            bbox: rect.bounding_box(),
            color: algo.biome_color(crimson_id),
            params: ShapeParams::from_rect(&rect),
        });
    }
    
    Ok(())
}
