//! 森林填充步骤

use crate::core::biome::{BiomeId, BIOME_UNASSIGNED};
use crate::core::geometry::{self, Rect, Shape, ShapeParams, ShapeRecord};
use crate::generation::algorithm::RuntimeContext;

use super::BiomeDivisionAlgorithm;

pub fn execute(algo: &BiomeDivisionAlgorithm, ctx: &mut RuntimeContext) -> Result<(), String> {
    let forest_id = algo.get_biome_id("forest")
        .ok_or("未找到 forest 环境定义")?;
    let desert_surface_id = algo.get_biome_id("desert")
        .ok_or("未找到 desert 环境定义")?;
    let crimson_id = algo.get_biome_id("crimson")
        .ok_or("未找到 crimson 环境定义")?;
    
    // 读取层级边界（在取可变借用之前）
    let layer_top = ctx.layer_start_px("surface").ok_or("未找到 surface 层级定义")? as i32;
    let layer_bottom = ctx.layer_end_px("underground").ok_or("未找到 underground 层级定义")? as i32;
    let scan_y = ((layer_top as u32) + (layer_bottom as u32)) / 2;
    
    let bm = ctx.biome_map.as_mut().ok_or("需先执行海洋生成")?;
    let w = bm.width as i32;
    let h = bm.height as i32;
    
    let threshold = algo.params.forest_fill_merge_threshold as i32;
    
    // ── 计算真沙漠槽位，排除其上方地表沙漠参与扩散 ──
    let mut true_desert_ranges: Vec<(i32, i32)> = Vec::new(); // (x_min, x_max)
    if let Some(slots) = ctx.shared.get("desert_true_slots")
        .and_then(|v| v.downcast_ref::<Vec<(i32, i32)>>())
    {
        for &(cx, sw) in slots {
            true_desert_ranges.push((cx - sw / 2, cx + sw / 2));
        }
    }
    
    // 判断一个沙漠区段是否是真沙漠候选（中心落在候选槽位范围内）
    let is_true_desert_candidate = |seg_start: i32, seg_end: i32| -> bool {
        let mid = (seg_start + seg_end) / 2;
        true_desert_ranges.iter().any(|&(lo, hi)| mid >= lo && mid <= hi)
    };
    
    let can_expand = |bid: BiomeId| -> bool {
        bid == desert_surface_id || bid == crimson_id
    };
    
    // ── 阶段 1：在 y=中间扫描线 扫描，判断哪些沙漠/猩红需要扩散 ──
    // scan_y 已在上方从层级配置计算
    
    struct Seg {
        biome: BiomeId,
        start: i32,
        end: i32,
    }
    let mut segs: Vec<Seg> = Vec::new();
    {
        let mut sx = 0i32;
        while sx < w {
            let bid = bm.get(sx as u32, scan_y);
            let seg_start = sx;
            while sx < w && bm.get(sx as u32, scan_y) == bid {
                sx += 1;
            }
            segs.push(Seg { biome: bid, start: seg_start, end: sx });
        }
    }
    
    // 记录扩散任务：(沙漠/猩红区段的边缘x, 方向, biome_id)
    // direction: -1=向左扩散, +1=向右扩散
    struct ExpandTask {
        edge_x: i32,    // 扩散起始边缘
        direction: i32, // -1 向左, +1 向右
        fill_id: BiomeId,
    }
    let mut tasks: Vec<ExpandTask> = Vec::new();
    let seg_count = segs.len();
    
    for i in 0..seg_count {
        if !can_expand(segs[i].biome) {
            continue;
        }
        // 真沙漠候选不参与扩散
        if segs[i].biome == desert_surface_id
            && is_true_desert_candidate(segs[i].start, segs[i].end)
        {
            continue;
        }
        let fill_id = segs[i].biome;
        
        // 左侧空隙检查
        if i >= 1 && segs[i - 1].biome == BIOME_UNASSIGNED {
            let gap_width = segs[i - 1].end - segs[i - 1].start;
            if gap_width < threshold {
                tasks.push(ExpandTask {
                    edge_x: segs[i].start, // 沙漠/猩红的左边缘
                    direction: -1,
                    fill_id,
                });
            }
        }
        
        // 右侧空隙检查
        if i + 1 < seg_count && segs[i + 1].biome == BIOME_UNASSIGNED {
            let gap_width = segs[i + 1].end - segs[i + 1].start;
            if gap_width < threshold {
                tasks.push(ExpandTask {
                    edge_x: segs[i].end, // 沙漠/猩红的右边缘
                    direction: 1,
                    fill_id,
                });
            }
        }
    }
    
    // ── 阶段 2：逐行从实际边缘向外扩散，直到碰到非空像素 ──
    for task in &tasks {
        for y in layer_top..layer_bottom {
            // 从扫描线的 edge_x 向内搜索，找到该行实际的沙漠/猩红边缘
            // 这样避免扫描线位置和实际边缘错位导致夹缝
            let inward = -task.direction; // 向内方向
            let mut actual_edge = task.edge_x;
            // 先向内找到属于 fill_id 的像素
            loop {
                if actual_edge < 0 || actual_edge >= w { break; }
                if bm.get(actual_edge as u32, y as u32) == task.fill_id {
                    break;
                }
                actual_edge += inward;
            }
            // 从实际边缘向外扩散
            let mut x = actual_edge;
            loop {
                x += task.direction;
                if x < 0 || x >= w { break; }
                if bm.get(x as u32, y as u32) != BIOME_UNASSIGNED {
                    break;
                }
                bm.set(x as u32, y as u32, task.fill_id);
            }
        }
    }
    
    // ── 阶段 3：剩余空白填充为森林（仅地表+地下层）—— geometry API
    let fill_rect = Rect::new(0, layer_top, w, layer_bottom);
    geometry::fill_biome_if(&fill_rect, bm, forest_id, |c| c == BIOME_UNASSIGNED);
    ctx.shape_log.push(ShapeRecord {
        label: "森林填充区域".into(),
        bbox: fill_rect.bounding_box(),
        color: algo.biome_color(forest_id),
        params: ShapeParams::from_rect(&fill_rect),
    });
    
    Ok(())
}
