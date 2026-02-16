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
    
    // TODO: 其他步骤参数
}

impl Default for BiomeDivisionParams {
    fn default() -> Self {
        Self {
            ocean_left_width: 0.10,
            ocean_right_width: 0.10,
            ocean_top_limit: 0.10,
            ocean_bottom_limit: 0.40,
            forest_width_ratio: 0.05,
            jungle_width_ratio: 0.16,
            jungle_top_limit: 0.10,
            jungle_bottom_limit: 0.85,
            jungle_center_offset_range: 0.20,
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

    /// 3. 雪原生成 — 在世界另一侧生成雪原（占位符）
    fn step_snow(&self, _ctx: &mut RuntimeContext) -> Result<(), String> {
        // TODO: 实现雪原生成逻辑
        Ok(())
    }

    /// 4. 沙漠生成 — 在世界空白区域随机生成沙漠（占位符）
    fn step_desert(&self, _ctx: &mut RuntimeContext) -> Result<(), String> {
        // TODO: 实现沙漠生成逻辑
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
                    default: serde_json::json!(0.10),
                    group: Some("海洋生成".to_string()),
                },
                ParamDef {
                    key: "ocean_right_width".to_string(),
                    name: "右侧海洋宽度".to_string(),
                    description: "右侧海洋占世界宽度的比例".to_string(),
                    param_type: ParamType::Float { min: 0.0, max: 1.0 },
                    default: serde_json::json!(0.10),
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
                    default: serde_json::json!(0.16),
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
