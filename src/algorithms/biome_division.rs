//! # 环境判定算法模块
//!
//! 实现 Phase 1：将世界划分为不同的环境区域。
//!
//! 这是一个独立的算法模块，通过 [`PhaseAlgorithm`] trait 向引擎声明自身。
//! 引擎不感知此模块内部逻辑，只通过 `meta()` / `execute()` / `get_params()` / `set_params()` 交互。

use serde::{Deserialize, Serialize};

use crate::core::biome::{BiomeDefinition, BiomeId, BiomeMap, BIOME_UNASSIGNED};
use crate::core::geometry::{self, Ellipse, Rect, Shape, ShapeCombine, Trapezoid};
use crate::generation::algorithm::{
    ParamDef, ParamType, PhaseAlgorithm, PhaseMeta, RuntimeContext, StepMeta,
};

/// 辅助：根据 key 查找 biome ID
fn biome_id_by_key(defs: &[BiomeDefinition], key: &str) -> Option<BiomeId> {
    defs.iter().find(|b| b.key == key).map(|b| b.id)
}

// ═══════════════════════════════════════════════════════════
// 算法参数
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeDivisionParams {
    // 海洋生成
    pub ocean_left_width: f64,
    pub ocean_right_width: f64,
    pub ocean_top_limit: f64,
    pub ocean_bottom_limit: f64,
    
    // 森林生成
    pub forest_width_ratio: f64,
    
    // 丛林生成
    pub jungle_width_ratio: f64,
    pub jungle_top_limit: f64,
    pub jungle_bottom_limit: f64,
    pub jungle_center_offset_range: f64,
    
    // 雪原生成
    pub snow_top_width_ratio: f64,
    pub snow_bottom_width_ratio: f64,
    pub snow_top_limit: f64,
    pub snow_bottom_limit: f64,
    pub snow_bottom_depth_factor: f64,
    pub snow_center_offset_range: f64,
    
    // 沙漠生成
    pub desert_surface_count: u32,
    pub desert_surface_width_min: f64,
    pub desert_surface_width_max: f64,
    pub desert_surface_top_limit: f64,
    pub desert_surface_bottom_limit: f64,
    pub desert_surface_min_spacing: f64,
    pub desert_true_count: u32,
    pub desert_true_top_limit: f64,
    pub desert_true_bottom_limit: f64,
    pub desert_true_depth_factor: f64,
    
    // 猩红生成
    pub crimson_count: u32,
    pub crimson_width_min: f64,
    pub crimson_width_max: f64,
    pub crimson_top_limit: f64,
    pub crimson_bottom_limit: f64,
    pub crimson_min_spacing: f64,
    
    // 森林填充
    pub forest_fill_merge_threshold: u32,
}

impl Default for BiomeDivisionParams {
    fn default() -> Self {
        Self {
            ocean_left_width: 0.05,
            ocean_right_width: 0.05,
            ocean_top_limit: 0.10,
            ocean_bottom_limit: 0.40,
            forest_width_ratio: 0.05,
            jungle_width_ratio: 0.12,
            jungle_top_limit: 0.10,
            jungle_bottom_limit: 0.85,
            jungle_center_offset_range: 0.20,
            snow_top_width_ratio: 0.08,
            snow_bottom_width_ratio: 0.20,
            snow_top_limit: 0.10,
            snow_bottom_limit: 0.85,
            snow_bottom_depth_factor: 0.8,
            snow_center_offset_range: 0.12,
            desert_surface_count: 3,
            desert_surface_width_min: 0.03,
            desert_surface_width_max: 0.05,
            desert_surface_top_limit: 0.10,
            desert_surface_bottom_limit: 0.40,
            desert_surface_min_spacing: 0.15,
            desert_true_count: 1,
            desert_true_top_limit: 0.30,
            desert_true_bottom_limit: 0.85,
            desert_true_depth_factor: 0.90,
            crimson_count: 3,
            crimson_width_min: 0.025,
            crimson_width_max: 0.1,
            crimson_top_limit: 0.10,
            crimson_bottom_limit: 0.40,
            crimson_min_spacing: 0.15,
            forest_fill_merge_threshold: 100,
        }
    }
}

// ═══════════════════════════════════════════════════════════
// 算法模块
// ═══════════════════════════════════════════════════════════

pub struct BiomeDivisionAlgorithm {
    /// 环境定义列表（用于运行时动态查找）
    biome_definitions: Vec<BiomeDefinition>,
    /// 可调参数
    params: BiomeDivisionParams,
}

impl BiomeDivisionAlgorithm {
    pub fn new(biome_definitions: &[BiomeDefinition]) -> Self {
        Self {
            biome_definitions: biome_definitions.to_vec(),
            params: BiomeDivisionParams::default(),
        }
    }
    
    /// 辅助方法：根据 key 查找 biome ID
    fn get_biome_id(&self, key: &str) -> Option<BiomeId> {
        biome_id_by_key(&self.biome_definitions, key)
    }

    // ── 各子步骤实现 ────────────────────────────────────────

    /// 0. 太空/地狱填充 — 初始化世界并填充太空层和地狱层
    fn step_space_hell(&self, ctx: &mut RuntimeContext) -> Result<(), String> {
        let space_id = self.get_biome_id("space")
            .ok_or("未找到 space 环境定义")?;
        let hell_id = self.get_biome_id("hell")
            .ok_or("未找到 hell 环境定义")?;
        
        let w = ctx.world.width as i32;
        let h = ctx.world.height as i32;
        
        // 初始化 BiomeMap（全部填充为 UNASSIGNED）
        *ctx.biome_map = Some(BiomeMap::new_filled(w as u32, h as u32, BIOME_UNASSIGNED));
        let bm = ctx.biome_map.as_mut().unwrap();
        
        // 太空层：0% ~ 10%
        let space_bottom = (h as f64 * 0.10) as i32;
        geometry::fill_biome(&Rect::new(0, 0, w, space_bottom), bm, space_id);
        
        // 地狱层：85% ~ 100%
        let hell_top = (h as f64 * 0.85) as i32;
        geometry::fill_biome(&Rect::new(0, hell_top, w, h), bm, hell_id);
        
        Ok(())
    }

    /// 1. 海洋生成 — 在世界两侧生成海洋区域
    fn step_ocean(&self, ctx: &mut RuntimeContext) -> Result<(), String> {
        let ocean_id = self.get_biome_id("ocean")
            .ok_or("未找到 ocean 环境定义")?;
        
        let bm = ctx.biome_map.as_mut().ok_or("需先执行太空/地狱填充")?;
        let w = bm.width as i32;
        let h = bm.height as i32;
        
        let y_top = (h as f64 * self.params.ocean_top_limit) as i32;
        let y_bottom = (h as f64 * self.params.ocean_bottom_limit) as i32;
        
        // 左侧海洋
        let left_width = (w as f64 * self.params.ocean_left_width) as i32;
        geometry::fill_biome(&Rect::new(0, y_top, left_width, y_bottom), bm, ocean_id);
        
        // 右侧海洋
        let right_width = (w as f64 * self.params.ocean_right_width) as i32;
        geometry::fill_biome(&Rect::new(w - right_width, y_top, w, y_bottom), bm, ocean_id);
        
        Ok(())
    }

    /// 1. 森林生成 — 在世界中心地表层生成矩形森林
    fn step_forest(&self, ctx: &mut RuntimeContext) -> Result<(), String> {
        let forest_id = self.get_biome_id("forest")
            .ok_or("未找到 forest 环境定义")?;
        
        let bm = ctx.biome_map.as_mut().ok_or("需先执行海洋生成")?;
        let w = bm.width as i32;
        let h = bm.height as i32;
        
        let y_top = (h as f64 * 0.10) as i32;
        let y_bottom = (h as f64 * 0.40) as i32;
        let center_x = w / 2;
        let half_width = (w as f64 * self.params.forest_width_ratio) as i32;
        
        let shape = Rect::new(
            center_x - half_width, y_top,
            center_x + half_width, y_bottom,
        );
        geometry::fill_biome_if(&shape, bm, forest_id, |c| c == BIOME_UNASSIGNED);
        
        Ok(())
    }

    /// 2. 丛林生成 — 在世界一侧生成椭圆丛林
    fn step_jungle(&self, ctx: &mut RuntimeContext) -> Result<(), String> {
        let jungle_id = self.get_biome_id("jungle")
            .ok_or("未找到 jungle 环境定义")?;
        
        let bm = ctx.biome_map.as_mut().ok_or("需先执行海洋生成")?;
        let w = bm.width as i32;
        let h = bm.height as i32;
        
        // 基于 RNG 随机选择左/右
        use rand::Rng;
        let place_on_left = ctx.rng.gen_bool(0.5);
        
        // 保存到 shared 供雪原生成使用
        ctx.shared.insert("jungle_on_left".into(), Box::new(place_on_left));
        
        // 计算森林边界（水平居中，半宽 = forest_width_ratio）
        let forest_center = w / 2;
        let forest_half_width = (w as f64 * self.params.forest_width_ratio) as i32;
        let forest_left = forest_center - forest_half_width;
        let forest_right = forest_center + forest_half_width;
        
        // 计算海洋边界
        let ocean_left_right = (w as f64 * self.params.ocean_left_width) as i32;
        let ocean_right_left = w - (w as f64 * self.params.ocean_right_width) as i32;
        
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
        let max_offset = (available_width as f64 * self.params.jungle_center_offset_range) as i32;
        let offset = ctx.rng.gen_range(-max_offset..=max_offset);
        let jungle_cx = jungle_cx_base + offset;
        
        // 丛林椭圆参数
        let jungle_rx = (w as f64 * self.params.jungle_width_ratio / 2.0) as i32;
        let jungle_cy = h / 2;  // 椭圆中心在世界垂直中心
        let jungle_ry = h / 2;  // 椭圆半径覆盖整个世界高度
        
        // 实际写入范围（裁剪）
        let top_y = (h as f64 * self.params.jungle_top_limit) as i32;
        let bottom_y = (h as f64 * self.params.jungle_bottom_limit) as i32;
        
        // 丛林椭圆 + y范围裁剪（椭圆 ∩ 矩形）
        let ell = Ellipse::new(
            jungle_cx as f64, jungle_cy as f64,
            jungle_rx as f64, jungle_ry as f64,
        );
        let clip = Rect::new(0, top_y, w, bottom_y);
        let shape = ell.intersect(clip);
        geometry::fill_biome_if(&shape, bm, jungle_id, |c| c == BIOME_UNASSIGNED);
        
        Ok(())
    }

    /// 3. 雪原生成 — 在世界另一侧生成梯形雪原（上窄下宽）
    fn step_snow(&self, ctx: &mut RuntimeContext) -> Result<(), String> {
        let snow_id = self.get_biome_id("snow")
            .ok_or("未找到 snow 环境定义")?;
        
        let bm = ctx.biome_map.as_mut().ok_or("需先执行海洋生成")?;
        let w = bm.width as i32;
        let h = bm.height as i32;
        
        // 从 shared 读取丛林位置，雪原在对侧
        use rand::Rng;
        let jungle_on_left = ctx.shared.get("jungle_on_left")
            .and_then(|v| v.downcast_ref::<bool>())
            .copied()
            .unwrap_or(false);
        let place_on_left = !jungle_on_left;
        
        // 计算森林边界
        let forest_center = w / 2;
        let forest_half_width = (w as f64 * self.params.forest_width_ratio) as i32;
        let forest_left = forest_center - forest_half_width;
        let forest_right = forest_center + forest_half_width;
        
        // 计算海洋边界
        let ocean_left_right = (w as f64 * self.params.ocean_left_width) as i32;
        let ocean_right_left = w - (w as f64 * self.params.ocean_right_width) as i32;
        
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
        let max_offset = (available_width as f64 * self.params.snow_center_offset_range) as i32;
        let offset = ctx.rng.gen_range(-max_offset..=max_offset);
        let snow_cx = snow_cx_base + offset;
        
        // 梯形参数（上窄下宽）
        let top_half_width = (w as f64 * self.params.snow_top_width_ratio / 2.0) as i32;
        let bottom_half_width = (w as f64 * self.params.snow_bottom_width_ratio / 2.0) as i32;
        
        let top_y = (h as f64 * self.params.snow_top_limit) as i32;
        let bottom_y = (h as f64 * self.params.snow_bottom_limit * self.params.snow_bottom_depth_factor) as i32;
        
        // 梯形填充（只替换 BIOME_UNASSIGNED）
        let shape = Trapezoid::new(
            top_y, bottom_y.min(h),
            (snow_cx - top_half_width) as f64,
            (snow_cx + top_half_width) as f64,
            (snow_cx - bottom_half_width) as f64,
            (snow_cx + bottom_half_width) as f64,
        );
        geometry::fill_biome_if(&shape, bm, snow_id, |c| c == BIOME_UNASSIGNED);
        
        Ok(())
    }

    /// 5. 沙漠生成 — 在世界空白区域随机生成沙漠地表和真沙漠
    ///
    /// 核心原则：所有环境互相避让，绝不重叠。
    ///
    /// 算法流程（先放真沙漠，再放普通地表）：
    ///   阶段 1：预计算 + 扫描空白区域
    ///   阶段 2：优先放置真沙漠（地表矩形 + 地下椭圆必须全空白）
    ///           从世界中心向两侧扫描，找到第一个满足条件的位置
    ///   阶段 3：放置剩余的普通地表沙漠（随机位置，避开已有沙漠）
    ///   阶段 4：一次性绘制所有沙漠 + 保存槽位信息到 shared
    fn step_desert(&self, ctx: &mut RuntimeContext) -> Result<(), String> {
        let desert_surface_id = self.get_biome_id("desert")
            .ok_or("未找到 desert 环境定义")?;
        let desert_true_id = self.get_biome_id("desert_true")
            .ok_or("未找到 desert_true 环境定义")?;
        
        let bm = ctx.biome_map.as_mut().ok_or("需先执行海洋生成")?;
        let w = bm.width as i32;
        let h = bm.height as i32;
        
        use rand::Rng;
        
        // ── 阶段 1：预计算常量 ─────────────────────────────
        let surface_top_y = (h as f64 * self.params.desert_surface_top_limit) as i32;
        let surface_bottom_y = (h as f64 * self.params.desert_surface_bottom_limit) as i32;
        let world_center_x = w / 2;
        
        // 真沙漠椭圆数学参数
        let true_top = h as f64 * self.params.desert_true_top_limit;
        let true_bottom = h as f64 * self.params.desert_true_bottom_limit
            * self.params.desert_true_depth_factor;
        let ell_cy = (true_top + true_bottom) / 2.0;
        let ell_ry = (true_bottom - true_top) / 2.0;
        let junction_y = h as f64 * self.params.desert_surface_bottom_limit;
        
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
        
        let surface_count = self.params.desert_surface_count as usize;
        let true_count = self.params.desert_true_count as usize;
        let min_spacing = (w as f64 * self.params.desert_surface_min_spacing) as i32;
        
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
        //
        // 策略：从世界中心向两侧交替扫描空白区段，
        // 对每个区段尝试在其中心放置，同时验证地表矩形 + 地下椭圆全空白。
        // 这保证真沙漠总是拿到离中心最近的有效位置。
        
        if true_count > 0 && ell_ry > 0.0 {
            let mut ranges_by_center: Vec<(i32, i32)> = empty_ranges.clone();
            ranges_by_center.sort_by_key(|&(s, e)| {
                let mid = (s + e) / 2;
                (mid - world_center_x).abs()
            });
            
            let mut true_placed = 0;
            
            for &(range_start, range_end) in &ranges_by_center {
                if true_placed >= true_count { break; }
                
                let avg_width_ratio = (self.params.desert_surface_width_min
                    + self.params.desert_surface_width_max) / 2.0;
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
                self.params.desert_surface_width_min..=self.params.desert_surface_width_max
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
            
            // 绘制真沙漠完整椭圆（覆写其内部的地表沙漠）—— geometry API
            if slot.has_true {
                let ell = Ellipse::new(slot.center_x as f64, ell_cy, slot.rx, ell_ry);
                geometry::fill_biome_if(&ell, bm, desert_true_id, |c| {
                    c == BIOME_UNASSIGNED || c == desert_surface_id
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

    /// 5. 猩红生成 — 在世界空白区域随机生成猩红矩形（逻辑同沙漠地表）
    fn step_crimson(&self, ctx: &mut RuntimeContext) -> Result<(), String> {
        let crimson_id = self.get_biome_id("crimson")
            .ok_or("未找到 crimson 环境定义")?;
        
        let bm = ctx.biome_map.as_mut().ok_or("需先执行海洋生成")?;
        let w = bm.width as i32;
        let h = bm.height as i32;
        
        use rand::Rng;
        
        let surface_top_y = (h as f64 * self.params.crimson_top_limit) as i32;
        let surface_bottom_y = (h as f64 * self.params.crimson_bottom_limit) as i32;
        let min_spacing = (w as f64 * self.params.crimson_min_spacing) as i32;
        let count = self.params.crimson_count as usize;
        
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
                self.params.crimson_width_min..=self.params.crimson_width_max
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
        for slot in &slots {
            let half_width = slot.width / 2;
            let xl = (slot.center_x - half_width).max(0);
            let xr = (slot.center_x + half_width).min(w);
            
            let rect = Rect::new(xl, surface_top_y, xr, surface_bottom_y.min(h));
            geometry::fill_biome_if(&rect, bm, crimson_id, |c| c == BIOME_UNASSIGNED);
        }
        
        Ok(())
    }

    /// 6. 森林填充 — 沙漠/猩红近距离扩散 + 剩余空白填森林
    ///
    /// 算法流程：
    ///   阶段 1：在 y=25% 扫描线判断沙漠/猩红左右是否有窄空隙（< 阈值）
    ///           记录需要扩散的方向（左/右）和对应环境 ID
    ///   阶段 2：对需要扩散的沙漠/猩红，逐行(y=10%..40%)从边缘
    ///           向外逐像素填充 UNASSIGNED，直到碰到非空像素
    ///           （自动适配梯形腰/椭圆弧等不规则边界）
    ///   阶段 3：剩余 UNASSIGNED 填充为森林（仅地表+地下层）
    fn step_forest_fill(&self, ctx: &mut RuntimeContext) -> Result<(), String> {
        let forest_id = self.get_biome_id("forest")
            .ok_or("未找到 forest 环境定义")?;
        let desert_surface_id = self.get_biome_id("desert")
            .ok_or("未找到 desert 环境定义")?;
        let crimson_id = self.get_biome_id("crimson")
            .ok_or("未找到 crimson 环境定义")?;
        
        let bm = ctx.biome_map.as_mut().ok_or("需先执行海洋生成")?;
        let w = bm.width as i32;
        let h = bm.height as i32;
        
        let threshold = self.params.forest_fill_merge_threshold as i32;
        
        let layer_top = (h as f64 * 0.10) as i32;
        let layer_bottom = (h as f64 * 0.40).min(h as f64) as i32;
        
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
        
        // ── 阶段 1：在 y=25% 扫描，判断哪些沙漠/猩红需要扩散 ──
        let scan_y = (h as f64 * 0.25) as u32;
        
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
        
        Ok(())
    }

    /// 8. 地块填充 — 将世界内所有剩余的 UNASSIGNED 区域填充为地块（石头）
    ///
    /// 主要填充洞穴层（40%-85%）中未被任何环境覆盖的区域。
    fn step_stone_fill(&self, ctx: &mut RuntimeContext) -> Result<(), String> {
        let stone_id = self.get_biome_id("stone")
            .ok_or("未找到 stone 环境定义")?;
        
        let bm = ctx.biome_map.as_mut().ok_or("需先执行前置步骤")?;
        let w = bm.width as i32;
        let h = bm.height as i32;
        
        let world_rect = Rect::new(0, 0, w, h);
        geometry::fill_biome_if(&world_rect, bm, stone_id, |c| c == BIOME_UNASSIGNED);
        
        Ok(())
    }

}

// ═══════════════════════════════════════════════════════════
// PhaseAlgorithm 实现
// ═══════════════════════════════════════════════════════════

impl PhaseAlgorithm for BiomeDivisionAlgorithm {
    fn meta(&self) -> PhaseMeta {
        PhaseMeta {
            id: "biome_division".to_string(),
            name: "环境判定".to_string(),
            description: "将世界划分为不同的环境区域（海洋、森林、丛林、雪原、沙漠、猩红）".to_string(),
            steps: vec![
                StepMeta {
                    display_index: 1,
                    name: "太空/地狱填充".to_string(),
                    description: "初始化世界并填充太空层(0-10%)和地狱层(85-100%)".to_string(),
                    doc_url: None,
                },
                StepMeta {
                    display_index: 2,
                    name: "海洋生成".to_string(),
                    description: "在世界两侧生成海洋区域".to_string(),
                    doc_url: None,
                },
                StepMeta {
                    display_index: 3,
                    name: "森林生成".to_string(),
                    description: "在世界中心生成森林".to_string(),
                    doc_url: None,
                },
                StepMeta {
                    display_index: 4,
                    name: "丛林生成".to_string(),
                    description: "在世界一侧生成丛林".to_string(),
                    doc_url: None,
                },
                StepMeta {
                    display_index: 5,
                    name: "雪原生成".to_string(),
                    description: "在世界另一侧生成雪原".to_string(),
                    doc_url: None,
                },
                StepMeta {
                    display_index: 6,
                    name: "沙漠生成".to_string(),
                    description: "在世界空白区域随机生成沙漠地表".to_string(),
                    doc_url: None,
                },
                StepMeta {
                    display_index: 7,
                    name: "猩红生成".to_string(),
                    description: "在世界空白区域随机生成猩红".to_string(),
                    doc_url: None,
                },
                StepMeta {
                    display_index: 8,
                    name: "森林填充".to_string(),
                    description: "沙漠/猩红扩散 + 剩余空白填充为森林".to_string(),
                    doc_url: None,
                },
                StepMeta {
                    display_index: 9,
                    name: "地块填充".to_string(),
                    description: "将所有剩余空白区域填充为岩石地块".to_string(),
                    doc_url: None,
                },
            ],
            params: vec![
                ParamDef {
                    key: "ocean_left_width".to_string(),
                    name: "左侧海洋宽度".to_string(),
                    description: "左侧海洋占世界宽度的比例".to_string(),
                    param_type: ParamType::Float { min: 0.0, max: 1.0 },
                    default: serde_json::json!(0.05),
                    group: Some("海洋生成".to_string()),
                },
                ParamDef {
                    key: "ocean_right_width".to_string(),
                    name: "右侧海洋宽度".to_string(),
                    description: "右侧海洋占世界宽度的比例".to_string(),
                    param_type: ParamType::Float { min: 0.0, max: 1.0 },
                    default: serde_json::json!(0.05),
                    group: Some("海洋生成".to_string()),
                },
                ParamDef {
                    key: "ocean_top_limit".to_string(),
                    name: "海洋上边界".to_string(),
                    description: "海洋区域上边界（世界高度百分比，0.0=顶部。地表层起点0.10）".to_string(),
                    param_type: ParamType::Float { min: 0.0, max: 1.0 },
                    default: serde_json::json!(0.10),
                    group: Some("海洋生成".to_string()),
                },
                ParamDef {
                    key: "ocean_bottom_limit".to_string(),
                    name: "海洋下边界".to_string(),
                    description: "海洋区域下边界（世界高度百分比，1.0=底部。地下层终点0.40）".to_string(),
                    param_type: ParamType::Float { min: 0.0, max: 1.0 },
                    default: serde_json::json!(0.40),
                    group: Some("海洋生成".to_string()),
                },
                ParamDef {
                    key: "forest_width_ratio".to_string(),
                    name: "森林宽度比例".to_string(),
                    description: "出生点森林的水平半宽（从中心向两侧延伸，相对世界宽度）".to_string(),
                    param_type: ParamType::Float { min: 0.0, max: 1.0 },
                    default: serde_json::json!(0.15),
                    group: Some("森林生成".to_string()),
                },
                ParamDef {
                    key: "jungle_width_ratio".to_string(),
                    name: "丛林宽度比例".to_string(),
                    description: "丛林椭圆的宽度（相对世界宽度）".to_string(),
                    param_type: ParamType::Float { min: 0.0, max: 1.0 },
                    default: serde_json::json!(0.12),
                    group: Some("丛林生成".to_string()),
                },
                ParamDef {
                    key: "jungle_top_limit".to_string(),
                    name: "丛林上边界".to_string(),
                    description: "丛林实际生成的顶部限制（0.0=世界顶，0.10=地表层顶）".to_string(),
                    param_type: ParamType::Float { min: 0.0, max: 1.0 },
                    default: serde_json::json!(0.10),
                    group: Some("丛林生成".to_string()),
                },
                ParamDef {
                    key: "jungle_bottom_limit".to_string(),
                    name: "丛林下边界".to_string(),
                    description: "丛林实际生成的底部限制（0.85=洞穴层底，1.0=地狱顶）".to_string(),
                    param_type: ParamType::Float { min: 0.0, max: 1.0 },
                    default: serde_json::json!(0.85),
                    group: Some("丛林生成".to_string()),
                },
                ParamDef {
                    key: "jungle_center_offset_range".to_string(),
                    name: "中心偏移范围".to_string(),
                    description: "丛林中心点在可用空间内的随机偏移范围（0.0=无偏移，0.15=±15%）".to_string(),
                    param_type: ParamType::Float { min: 0.0, max: 0.5 },
                    default: serde_json::json!(0.20),
                    group: Some("丛林生成".to_string()),
                },                ParamDef {
                    key: "snow_top_width_ratio".to_string(),
                    name: "雪原上边宽度".to_string(),
                    description: "雪原梯形上边宽度（地表层，相对世界宽度）".to_string(),
                    param_type: ParamType::Float { min: 0.0, max: 1.0 },
                    default: serde_json::json!(0.08),
                    group: Some("雪原生成".to_string()),
                },
                ParamDef {
                    key: "snow_bottom_width_ratio".to_string(),
                    name: "雪原下边宽度".to_string(),
                    description: "雪原梯形下边宽度（洞穴层，相对世界宽度）".to_string(),
                    param_type: ParamType::Float { min: 0.0, max: 1.0 },
                    default: serde_json::json!(0.20),
                    group: Some("雪原生成".to_string()),
                },
                ParamDef {
                    key: "snow_top_limit".to_string(),
                    name: "雪原上边界".to_string(),
                    description: "雪原顶部边界（0.0=世界顶，0.10=地表层顶）".to_string(),
                    param_type: ParamType::Float { min: 0.0, max: 1.0 },
                    default: serde_json::json!(0.1),
                    group: Some("雪原生成".to_string()),
                },
                ParamDef {
                    key: "snow_bottom_limit".to_string(),
                    name: "雪原下边界".to_string(),
                    description: "雪原底部边界（用于计算实际深度，0.85=洞穴层底）".to_string(),
                    param_type: ParamType::Float { min: 0.0, max: 1.0 },
                    default: serde_json::json!(0.85),
                    group: Some("雪原生成".to_string()),
                },
                ParamDef {
                    key: "snow_bottom_depth_factor".to_string(),
                    name: "雪原深度因子".to_string(),
                    description: "控制雪原向下延伸深度（实际深度 = 下边界 × 深度因子，避免触及地狱）".to_string(),
                    param_type: ParamType::Float { min: 0.0, max: 1.0 },
                    default: serde_json::json!(0.8),
                    group: Some("雪原生成".to_string()),
                },
                ParamDef {
                    key: "snow_center_offset_range".to_string(),
                    name: "中心偏移范围".to_string(),
                    description: "雪原中心点在可用空间内的随机偏移范围（0.0=无偏移，0.12=±12%）".to_string(),
                    param_type: ParamType::Float { min: 0.0, max: 0.5 },
                    default: serde_json::json!(0.12),
                    group: Some("雪原生成".to_string()),
                },
                ParamDef {
                    key: "desert_surface_count".to_string(),
                    name: "沙漠地表数量".to_string(),
                    description: "生成的沙漠地表区域数量".to_string(),
                    param_type: ParamType::Int { min: 0, max: 10 },
                    default: serde_json::json!(3),
                    group: Some("沙漠生成".to_string()),
                },
                ParamDef {
                    key: "desert_surface_width_min".to_string(),
                    name: "沙漠地表最小宽度".to_string(),
                    description: "沙漠地表矩形最小宽度（相对世界宽度）".to_string(),
                    param_type: ParamType::Float { min: 0.0, max: 1.0 },
                    default: serde_json::json!(0.025),
                    group: Some("沙漠生成".to_string()),
                },
                ParamDef {
                    key: "desert_surface_width_max".to_string(),
                    name: "沙漠地表最大宽度".to_string(),
                    description: "沙漠地表矩形最大宽度（相对世界宽度）".to_string(),
                    param_type: ParamType::Float { min: 0.0, max: 1.0 },
                    default: serde_json::json!(0.05),
                    group: Some("沙漠生成".to_string()),
                },
                ParamDef {
                    key: "desert_surface_top_limit".to_string(),
                    name: "沙漠地表上边界".to_string(),
                    description: "沙漠地表顶部边界（0.10=地表层顶）".to_string(),
                    param_type: ParamType::Float { min: 0.0, max: 1.0 },
                    default: serde_json::json!(0.10),
                    group: Some("沙漠生成".to_string()),
                },
                ParamDef {
                    key: "desert_surface_bottom_limit".to_string(),
                    name: "沙漠地表下边界".to_string(),
                    description: "沙漠地表底部边界（0.40=地下层底）".to_string(),
                    param_type: ParamType::Float { min: 0.0, max: 1.0 },
                    default: serde_json::json!(0.40),
                    group: Some("沙漠生成".to_string()),
                },
                ParamDef {
                    key: "desert_surface_min_spacing".to_string(),
                    name: "沙漠地表最小间距".to_string(),
                    description: "相邻沙漠地表之间的最小间距（相对世界宽度）".to_string(),
                    param_type: ParamType::Float { min: 0.0, max: 1.0 },
                    default: serde_json::json!(0.15),
                    group: Some("沙漠生成".to_string()),
                },
                ParamDef {
                    key: "desert_true_count".to_string(),
                    name: "真沙漠数量".to_string(),
                    description: "生成的真沙漠数量，选择离中心最近的地表沙漠".to_string(),
                    param_type: ParamType::Int { min: 0, max: 5 },
                    default: serde_json::json!(1),
                    group: Some("沙漠生成".to_string()),
                },
                ParamDef {
                    key: "desert_true_top_limit".to_string(),
                    name: "真沙漠上边界".to_string(),
                    description: "真沙漠椭圆顶部（0.30=地下层顶）".to_string(),
                    param_type: ParamType::Float { min: 0.0, max: 1.0 },
                    default: serde_json::json!(0.30),
                    group: Some("沙漠生成".to_string()),
                },
                ParamDef {
                    key: "desert_true_bottom_limit".to_string(),
                    name: "真沙漠下边界".to_string(),
                    description: "真沙漠底部边界基准（0.85=洞穴层底）".to_string(),
                    param_type: ParamType::Float { min: 0.0, max: 1.0 },
                    default: serde_json::json!(0.85),
                    group: Some("沙漠生成".to_string()),
                },
                ParamDef {
                    key: "desert_true_depth_factor".to_string(),
                    name: "真沙漠深度因子".to_string(),
                    description: "控制真沙漠向下延伸深度（实际深度 = 下边界 × 深度因子）".to_string(),
                    param_type: ParamType::Float { min: 0.0, max: 1.0 },
                    default: serde_json::json!(0.90),
                    group: Some("沙漠生成".to_string()),
                },
                ParamDef {
                    key: "crimson_count".to_string(),
                    name: "猩红数量".to_string(),
                    description: "生成的猩红区域数量".to_string(),
                    param_type: ParamType::Int { min: 0, max: 10 },
                    default: serde_json::json!(3),
                    group: Some("猩红生成".to_string()),
                },
                ParamDef {
                    key: "crimson_width_min".to_string(),
                    name: "猩红最小宽度".to_string(),
                    description: "猩红矩形最小宽度（相对世界宽度）".to_string(),
                    param_type: ParamType::Float { min: 0.0, max: 1.0 },
                    default: serde_json::json!(0.025),
                    group: Some("猩红生成".to_string()),
                },
                ParamDef {
                    key: "crimson_width_max".to_string(),
                    name: "猩红最大宽度".to_string(),
                    description: "猩红矩形最大宽度（相对世界宽度）".to_string(),
                    param_type: ParamType::Float { min: 0.0, max: 1.0 },
                    default: serde_json::json!(0.05),
                    group: Some("猩红生成".to_string()),
                },
                ParamDef {
                    key: "crimson_top_limit".to_string(),
                    name: "猩红上边界".to_string(),
                    description: "猩红顶部边界（0.10=地表层顶）".to_string(),
                    param_type: ParamType::Float { min: 0.0, max: 1.0 },
                    default: serde_json::json!(0.10),
                    group: Some("猩红生成".to_string()),
                },
                ParamDef {
                    key: "crimson_bottom_limit".to_string(),
                    name: "猩红下边界".to_string(),
                    description: "猩红底部边界（0.40=地下层底）".to_string(),
                    param_type: ParamType::Float { min: 0.0, max: 1.0 },
                    default: serde_json::json!(0.40),
                    group: Some("猩红生成".to_string()),
                },
                ParamDef {
                    key: "crimson_min_spacing".to_string(),
                    name: "猩红最小间距".to_string(),
                    description: "相邻猩红之间的最小间距（相对世界宽度）".to_string(),
                    param_type: ParamType::Float { min: 0.0, max: 1.0 },
                    default: serde_json::json!(0.15),
                    group: Some("猩红生成".to_string()),
                },
                ParamDef {
                    key: "forest_fill_merge_threshold".to_string(),
                    name: "扩散阈值".to_string(),
                    description: "沙漠/猩红边缘到邻居环境的空隙小于此像素数时，扩散填充而非生成森林".to_string(),
                    param_type: ParamType::Int { min: 0, max: 500 },
                    default: serde_json::json!(100),
                    group: Some("森林填充".to_string()),
                },
            ],
        }
    }

    fn execute(&mut self, step_index: usize, ctx: &mut RuntimeContext) -> Result<(), String> {
        match step_index {
            0 => self.step_space_hell(ctx),
            1 => self.step_ocean(ctx),
            2 => self.step_forest(ctx),
            3 => self.step_jungle(ctx),
            4 => self.step_snow(ctx),
            5 => self.step_desert(ctx),
            6 => self.step_crimson(ctx),
            7 => self.step_forest_fill(ctx),
            8 => self.step_stone_fill(ctx),
            _ => Err(format!("无效步骤索引: {step_index}")),
        }
    }

    fn get_params(&self) -> serde_json::Value {
        serde_json::to_value(&self.params).unwrap_or_default()
    }

    fn set_params(&mut self, params: &serde_json::Value) {
        if let Ok(p) = serde_json::from_value::<BiomeDivisionParams>(params.clone()) {
            self.params = p;
        }
    }

    fn on_reset(&mut self) {
        // 无需清理运行时状态（当前无跨步骤状态）
    }
}
