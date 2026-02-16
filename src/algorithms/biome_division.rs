//! # 环境判定算法模块
//!
//! 实现 Phase 1：将世界划分为不同的环境区域。
//!
//! 这是一个独立的算法模块，通过 [`PhaseAlgorithm`] trait 向引擎声明自身。
//! 引擎不感知此模块内部逻辑，只通过 `meta()` / `execute()` / `get_params()` / `set_params()` 交互。

use serde::{Deserialize, Serialize};

use crate::core::biome::{BiomeDefinition, BiomeId, BiomeMap, BIOME_UNASSIGNED};
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
    
    // TODO: 其他步骤参数
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
            desert_surface_width_min: 0.025,
            desert_surface_width_max: 0.05,
            desert_surface_top_limit: 0.10,
            desert_surface_bottom_limit: 0.40,
            desert_surface_min_spacing: 0.15,
            desert_true_count: 1,
            desert_true_top_limit: 0.30,
            desert_true_bottom_limit: 0.85,
            desert_true_depth_factor: 0.90,
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

    /// 0. 海洋生成 — 在世界两侧生成海洋区域
    fn step_ocean(&self, ctx: &mut RuntimeContext) -> Result<(), String> {
        let ocean_id = self.get_biome_id("ocean")
            .ok_or("未找到 ocean 环境定义")?;
        
        let w = ctx.world.width;
        let h = ctx.world.height;
        
        // 初始化 BiomeMap（全部填充为 UNASSIGNED）
        *ctx.biome_map = Some(BiomeMap::new_filled(w, h, BIOME_UNASSIGNED));
        let bm = ctx.biome_map.as_mut().unwrap();
        
        // 计算垂直范围（基于世界高度百分比）
        let y_top = (h as f64 * self.params.ocean_top_limit) as u32;
        let y_bottom = (h as f64 * self.params.ocean_bottom_limit) as u32;
        
        // 左侧海洋
        let left_width = (w as f64 * self.params.ocean_left_width) as u32;
        bm.fill_rect(0, y_top, left_width, y_bottom, ocean_id);
        
        // 右侧海洋
        let right_width = (w as f64 * self.params.ocean_right_width) as u32;
        bm.fill_rect(w - right_width, y_top, w, y_bottom, ocean_id);
        
        Ok(())
    }

    /// 1. 森林生成 — 在世界中心地表层生成矩形森林
    fn step_forest(&self, ctx: &mut RuntimeContext) -> Result<(), String> {
        let forest_id = self.get_biome_id("forest")
            .ok_or("未找到 forest 环境定义")?;
        
        let bm = ctx.biome_map.as_mut().ok_or("需先执行海洋生成")?;
        let w = bm.width;
        let h = bm.height;
        
        // 地表层范围：10% - 30%（world.json定义）
        let y_top = (h as f64 * 0.10) as u32;
        let y_bottom = (h as f64 * 0.30) as u32;
        
        // 水平中心区域：从中心向两侧延伸
        let center_x = w / 2;
        let half_width = (w as f64 * self.params.forest_width_ratio) as u32;
        let x_left = center_x.saturating_sub(half_width);
        let x_right = (center_x + half_width).min(w);
        
        // 填充矩形森林区域（只替换空白区域）
        for y in y_top..y_bottom {
            for x in x_left..x_right {
                if bm.get(x, y) == BIOME_UNASSIGNED {
                    bm.set(x, y, forest_id);
                }
            }
        }
        
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
        
        // 手动实现椭圆填充 + y范围裁剪
        let x0 = (jungle_cx - jungle_rx).max(0);
        let x1 = (jungle_cx + jungle_rx).min(w);
        let y0 = (jungle_cy - jungle_ry).max(0);
        let y1 = (jungle_cy + jungle_ry).min(h);
        
        for y in y0..y1 {
            // y 范围裁剪
            if y < top_y || y >= bottom_y {
                continue;
            }
            
            for x in x0..x1 {
                let current_id = bm.get(x as u32, y as u32);
                if current_id != BIOME_UNASSIGNED {
                    continue;
                }
                
                // 椭圆方程判定
                let dx = (x - jungle_cx) as f64 / jungle_rx as f64;
                let dy = (y - jungle_cy) as f64 / jungle_ry as f64;
                if dx * dx + dy * dy <= 1.0 {
                    bm.set(x as u32, y as u32, jungle_id);
                }
            }
        }
        
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
        
        // 手动实现梯形填充 + 条件判断（只替换 BIOME_UNASSIGNED）
        if top_y >= bottom_y {
            return Ok(());
        }
        
        let trap_h = (bottom_y - top_y) as f64;
        for y in top_y..bottom_y.min(h) {
            let t = (y - top_y) as f64 / trap_h;
            
            // 线性插值计算当前 y 的左右边界
            let top_left = (snow_cx - top_half_width) as f64;
            let top_right = (snow_cx + top_half_width) as f64;
            let bottom_left = (snow_cx - bottom_half_width) as f64;
            let bottom_right = (snow_cx + bottom_half_width) as f64;
            
            let left = top_left + (bottom_left - top_left) * t;
            let right = top_right + (bottom_right - top_right) * t;
            
            let xl = (left.floor().max(0.0)) as i32;
            let xr = (right.ceil().min(w as f64)) as i32;
            
            for x in xl..xr {
                let current_id = bm.get(x as u32, y as u32);
                if current_id == BIOME_UNASSIGNED {
                    bm.set(x as u32, y as u32, snow_id);
                }
            }
        }
        
        Ok(())
    }

    /// 4. 沙漠生成 — 在世界空白区域随机生成沙漠地表和真沙漠
    ///
    /// 核心原则：所有环境互相避让，绝不重叠。
    ///
    /// 算法流程（先放真沙漠，再放普通地表）：
    ///   阶段 1：预计算 + 扫描空白区域
    ///   阶段 2：优先放置真沙漠（地表矩形 + 地下椭圆必须全空白）
    ///           从世界中心向两侧扫描，找到第一个满足条件的位置
    ///   阶段 3：放置剩余的普通地表沙漠（随机位置，避开已有沙漠）
    ///   阶段 4：一次性绘制所有沙漠
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
        
        // ── 辅助：验证矩形区域全空白（采样步长 2，更精确）──
        let rect_all_empty = |bm: &BiomeMap, xl: i32, xr: i32, yt: i32, yb: i32| -> bool {
            let step = 2;
            let mut y = yt;
            while y < yb {
                let mut x = xl;
                while x < xr {
                    if bm.get(x as u32, y as u32) != BIOME_UNASSIGNED {
                        return false;
                    }
                    x += step;
                }
                y += step;
            }
            true
        };
        
        // ── 辅助：验证椭圆区域（从 true_top 到 true_bottom）全空白 ──
        let ellipse_all_empty = |bm: &BiomeMap, cx: i32, rx: f64| -> bool {
            if ell_ry <= 0.0 { return true; }
            let x0 = ((cx as f64 - rx).floor().max(0.0)) as i32;
            let x1 = ((cx as f64 + rx).ceil().min(w as f64)) as i32;
            let y0 = (true_top.floor().max(0.0)) as i32;
            let y1 = (true_bottom.ceil().min(h as f64)) as i32;
            let step = 2;
            let mut sy = y0;
            while sy < y1 {
                let mut sx = x0;
                while sx < x1 {
                    let dxe = (sx - cx) as f64 / rx;
                    let dye = (sy as f64 - ell_cy) / ell_ry;
                    if dxe * dxe + dye * dye <= 1.0 {
                        if bm.get(sx as u32, sy as u32) != BIOME_UNASSIGNED {
                            return false;
                        }
                    }
                    sx += step;
                }
                sy += step;
            }
            true
        };
        
        // ── 辅助：计算椭圆 rx ──────────────────────────────
        let compute_rx = |surface_half_width: f64| -> Option<f64> {
            if ell_ry <= 0.0 { return None; }
            let dy = (junction_y - ell_cy) / ell_ry;
            let dy_sq = dy * dy;
            if dy_sq >= 1.0 { return None; }
            Some(surface_half_width / (1.0 - dy_sq).sqrt())
        };
        
        // ── 沙漠槽位数据结构 ───────────────────────────────
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
        
        // ── 辅助：检查与已有沙漠的间距 ─────────────────────
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
            // 将空白区段按其中心离世界中心的距离排序
            let mut ranges_by_center: Vec<(i32, i32)> = empty_ranges.clone();
            ranges_by_center.sort_by_key(|&(s, e)| {
                let mid = (s + e) / 2;
                (mid - world_center_x).abs()
            });
            
            let mut true_placed = 0;
            
            for &(range_start, range_end) in &ranges_by_center {
                if true_placed >= true_count { break; }
                
                // 在此区段内，从离中心最近的位置开始尝试
                let avg_width_ratio = (self.params.desert_surface_width_min
                    + self.params.desert_surface_width_max) / 2.0;
                let width = (w as f64 * avg_width_ratio) as i32;
                let half_width = width / 2;
                
                if range_end - range_start < width { continue; }
                
                let min_cx = range_start + half_width;
                let max_cx = range_end - half_width;
                if min_cx >= max_cx { continue; }
                
                // 在区段内按离中心距离升序尝试多个位置
                // 生成候选位置列表：中心 → 左 → 右 → 更左 → 更右...
                let range_mid = (min_cx + max_cx) / 2;
                let closest_to_center = world_center_x.clamp(min_cx, max_cx);
                let scan_step = (width / 2).max(4); // 扫描步长
                
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
                    
                    // 检查间距
                    if !spacing_ok(&slots, cx, width, min_spacing) { continue; }
                    
                    // 验证地表矩形全空白
                    let xl = (cx - half_width).max(0);
                    let xr = (cx + half_width).min(w);
                    if !rect_all_empty(bm, xl, xr, surface_top_y, surface_bottom_y.min(h)) {
                        continue;
                    }
                    
                    // 计算 rx 并验证椭圆区域全空白
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
        
        // ── 阶段 4：一次性绘制 ─────────────────────────────
        
        for slot in &slots {
            let half_width = slot.width / 2;
            let xl = (slot.center_x - half_width).max(0);
            let xr = (slot.center_x + half_width).min(w);
            
            // 绘制地表沙漠矩形
            for y in surface_top_y..surface_bottom_y.min(h) {
                for x in xl..xr {
                    if bm.get(x as u32, y as u32) == BIOME_UNASSIGNED {
                        bm.set(x as u32, y as u32, desert_surface_id);
                    }
                }
            }
            
            // 绘制真沙漠完整椭圆（覆写其内部的地表沙漠）
            if slot.has_true {
                let rx = slot.rx;
                let x0 = ((slot.center_x as f64 - rx).floor().max(0.0)) as i32;
                let x1 = ((slot.center_x as f64 + rx).ceil().min(w as f64)) as i32;
                let y0 = (true_top.floor().max(0.0)) as i32;
                let y1 = (true_bottom.ceil().min(h as f64)) as i32;
                
                for y in y0..y1 {
                    for x in x0..x1 {
                        let cid = bm.get(x as u32, y as u32);
                        if cid == BIOME_UNASSIGNED || cid == desert_surface_id {
                            let dxe = (x - slot.center_x) as f64 / rx;
                            let dye = (y as f64 - ell_cy) / ell_ry;
                            if dxe * dxe + dye * dye <= 1.0 {
                                bm.set(x as u32, y as u32, desert_true_id);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    /// 5. 猩红生成 — 在世界空白/沙漠区域随机生成猩红（占位符）
    fn step_crimson(&self, _ctx: &mut RuntimeContext) -> Result<(), String> {
        // TODO: 实现猩红生成逻辑
        Ok(())
    }

    /// 6. 森林填充 — 将所有剩余空白区块变成森林（占位符）
    fn step_forest_fill(&self, _ctx: &mut RuntimeContext) -> Result<(), String> {
        // TODO: 实现森林填充逻辑
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
                    name: "海洋生成".to_string(),
                    description: "在世界两侧生成海洋区域".to_string(),
                    doc_url: None,
                },
                StepMeta {
                    name: "森林生成".to_string(),
                    description: "在世界中心生成森林".to_string(),
                    doc_url: None,
                },
                StepMeta {
                    name: "丛林生成".to_string(),
                    description: "在世界一侧生成丛林".to_string(),
                    doc_url: None,
                },
                StepMeta {
                    name: "雪原生成".to_string(),
                    description: "在世界另一侧生成雪原".to_string(),
                    doc_url: None,
                },
                StepMeta {
                    name: "沙漠生成".to_string(),
                    description: "在世界空白区域随机生成沙漠".to_string(),
                    doc_url: None,
                },
                StepMeta {
                    name: "猩红生成".to_string(),
                    description: "在世界空白/沙漠区域随机生成猩红".to_string(),
                    doc_url: None,
                },
                StepMeta {
                    name: "森林填充".to_string(),
                    description: "将所有剩余空白区块变成森林".to_string(),
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
            ],
        }
    }

    fn execute(&mut self, step_index: usize, ctx: &mut RuntimeContext) -> Result<(), String> {
        match step_index {
            0 => self.step_ocean(ctx),
            1 => self.step_forest(ctx),
            2 => self.step_jungle(ctx),
            3 => self.step_snow(ctx),
            4 => self.step_desert(ctx),
            5 => self.step_crimson(ctx),
            6 => self.step_forest_fill(ctx),
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
