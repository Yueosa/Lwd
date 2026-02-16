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

    /// 2. 丛林生成 — 在世界一侧生成丛林（占位符）
    fn step_jungle(&self, _ctx: &mut RuntimeContext) -> Result<(), String> {
        // TODO: 实现丛林生成逻辑
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
