//! # 环境判定算法模块
//!
//! 实现 Phase 1：将世界划分为不同的环境区域（海洋、森林、丛林、雪原、沙漠）。
//!
//! 这是一个独立的算法模块，通过 [`PhaseAlgorithm`] trait 向引擎声明自身。
//! 引擎不感知此模块内部逻辑，只通过 `meta()` / `execute()` / `get_params()` / `set_params()` 交互。

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::core::biome::{BiomeDefinition, BiomeId, BiomeMap};
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
    /// 每侧海洋占世界宽度的比例
    pub ocean_ratio: f64,
    /// 出生点森林半宽比例
    pub spawn_width_ratio: f64,
    /// 森林扩展最小比例
    pub expand_min_ratio: f64,
    /// 森林扩展最大比例
    pub expand_max_ratio: f64,
    /// 丛林宽度最小比例
    pub jungle_width_min: f64,
    /// 丛林宽度最大比例
    pub jungle_width_max: f64,
    /// 雪原顶部宽度比例
    pub snow_top_ratio: f64,
    /// 雪原底部宽度比例
    pub snow_bot_ratio: f64,
    /// 沙漠最少数量
    pub desert_min_count: u32,
    /// 沙漠最多数量
    pub desert_max_count: u32,
    /// 沙漠宽度比例范围
    pub desert_width_min: f64,
    pub desert_width_max: f64,
}

impl Default for BiomeDivisionParams {
    fn default() -> Self {
        Self {
            ocean_ratio: 0.10,
            spawn_width_ratio: 0.05,
            expand_min_ratio: 0.08,
            expand_max_ratio: 0.15,
            jungle_width_min: 0.12,
            jungle_width_max: 0.18,
            snow_top_ratio: 0.08,
            snow_bot_ratio: 0.16,
            desert_min_count: 1,
            desert_max_count: 3,
            desert_width_min: 0.05,
            desert_width_max: 0.10,
        }
    }
}

// ═══════════════════════════════════════════════════════════
// 算法模块
// ═══════════════════════════════════════════════════════════

pub struct BiomeDivisionAlgorithm {
    /// 各种 biome 的 ID（从 biome_definitions 查表得到）
    forest_id: BiomeId,
    ocean_id: BiomeId,
    jungle_id: BiomeId,
    snow_id: BiomeId,
    desert_id: BiomeId,
    /// 可调参数
    params: BiomeDivisionParams,
    /// 运行时状态：丛林在左侧还是右侧（via shared state）
    jungle_on_left: Option<bool>,
}

impl BiomeDivisionAlgorithm {
    pub fn new(biome_definitions: &[BiomeDefinition]) -> Self {
        Self {
            forest_id: biome_id_by_key(biome_definitions, "forest").unwrap_or(2),
            ocean_id: biome_id_by_key(biome_definitions, "ocean").unwrap_or(1),
            jungle_id: biome_id_by_key(biome_definitions, "jungle").unwrap_or(5),
            snow_id: biome_id_by_key(biome_definitions, "snow").unwrap_or(4),
            desert_id: biome_id_by_key(biome_definitions, "desert").unwrap_or(3),
            params: BiomeDivisionParams::default(),
            jungle_on_left: None,
        }
    }

    // ── 各子步骤实现 ────────────────────────────────────────

    /// 0. 将全世界初始化为森林
    fn step_forest_init(&self, ctx: &mut RuntimeContext) -> Result<(), String> {
        let w = ctx.world.width;
        let h = ctx.world.height;
        *ctx.biome_map = Some(BiomeMap::new_filled(w, h, self.forest_id));
        Ok(())
    }

    /// 1. 海洋边界
    fn step_ocean_border(&self, ctx: &mut RuntimeContext) -> Result<(), String> {
        let bm = ctx.biome_map.as_mut().ok_or("需先执行森林初始化")?;
        let w = bm.width;
        let h = bm.height;
        let border_w = (w as f64 * self.params.ocean_ratio) as u32;
        bm.fill_rect(0, 0, border_w, h, self.ocean_id);
        bm.fill_rect(w - border_w, 0, w, h, self.ocean_id);
        Ok(())
    }

    /// 2. 出生点森林
    fn step_spawn_forest(&self, ctx: &mut RuntimeContext) -> Result<(), String> {
        let bm = ctx.biome_map.as_mut().ok_or("需先执行森林初始化")?;
        let w = bm.width as f64;
        let h = bm.height as f64;
        let cx = w / 2.0;
        let cy = h / 2.0;
        let rx = w * self.params.spawn_width_ratio;
        let ry = h * 0.5;
        bm.fill_ellipse(cx, cy, rx, ry, self.forest_id);
        Ok(())
    }

    /// 3. 森林扩展
    fn step_forest_expand(&self, ctx: &mut RuntimeContext) -> Result<(), String> {
        let bm = ctx.biome_map.as_mut().ok_or("需先执行森林初始化")?;
        let w = bm.width as f64;
        let h = bm.height as f64;
        let cx = w / 2.0;
        let cy = h / 2.0;
        let expand_ratio = ctx
            .rng
            .gen_range(self.params.expand_min_ratio..=self.params.expand_max_ratio);
        let rx = w * expand_ratio;
        let ry = h * 0.5;
        bm.fill_ellipse(cx, cy, rx, ry, self.forest_id);
        Ok(())
    }

    /// 4. 丛林生成
    fn step_jungle(&mut self, ctx: &mut RuntimeContext) -> Result<(), String> {
        let jungle_on_left: bool = ctx.rng.r#gen();
        self.jungle_on_left = Some(jungle_on_left);

        let bm = ctx.biome_map.as_mut().ok_or("需先执行森林初始化")?;
        let w = bm.width as f64;
        let h = bm.height as f64;
        let jungle_width_ratio = ctx
            .rng
            .gen_range(self.params.jungle_width_min..=self.params.jungle_width_max);
        let rx = w * jungle_width_ratio / 2.0;
        let ry = h * 0.5;
        let ocean_w = w * self.params.ocean_ratio;
        let cx = if jungle_on_left {
            ocean_w + rx
        } else {
            w - ocean_w - rx
        };
        let cy = h / 2.0;
        bm.fill_ellipse_if(cx, cy, rx, ry, self.jungle_id, self.forest_id);
        Ok(())
    }

    /// 5. 雪原生成（梯形）
    fn step_snow(&self, ctx: &mut RuntimeContext) -> Result<(), String> {
        let snow_on_left = !self.jungle_on_left.unwrap_or(true);
        let bm = ctx.biome_map.as_mut().ok_or("需先执行森林初始化")?;
        let w = bm.width as f64;
        let h = bm.height as f64;
        let ocean_w = w * self.params.ocean_ratio;

        let top_half = w * self.params.snow_top_ratio / 2.0;
        let bot_half = w * self.params.snow_bot_ratio / 2.0;

        let center_x = if snow_on_left {
            ocean_w + bot_half
        } else {
            w - ocean_w - bot_half
        };

        let top_x0 = center_x - top_half;
        let top_x1 = center_x + top_half;
        let bot_x0 = center_x - bot_half;
        let bot_x1 = center_x + bot_half;

        let y_bot = bm.height;
        let hh = y_bot as f64;

        for y in 0..y_bot {
            let t = y as f64 / hh;
            let left = top_x0 + (bot_x0 - top_x0) * t;
            let right = top_x1 + (bot_x1 - top_x1) * t;
            let xl = (left.floor().max(0.0)) as u32;
            let xr = (right.ceil().min(w)) as u32;
            for x in xl..xr {
                if bm.get(x, y) == self.forest_id {
                    bm.set(x, y, self.snow_id);
                }
            }
        }
        Ok(())
    }

    /// 6. 沙漠判定
    fn step_desert(&self, ctx: &mut RuntimeContext) -> Result<(), String> {
        let bm = ctx.biome_map.as_mut().ok_or("需先执行森林初始化")?;
        let w = bm.width as f64;
        let h = bm.height as f64;
        let count = ctx
            .rng
            .gen_range(self.params.desert_min_count..=self.params.desert_max_count);

        for _ in 0..count {
            let ocean_margin = w * 0.15;
            let cx = ctx.rng.gen_range(ocean_margin..(w - ocean_margin));
            let cy = h / 2.0 + ctx.rng.gen_range(-(h * 0.2)..(h * 0.2));
            let rx = w
                * ctx
                    .rng
                    .gen_range(self.params.desert_width_min..=self.params.desert_width_max);
            let ry = h * ctx.rng.gen_range(0.15..=0.35);
            bm.fill_ellipse_if(cx, cy, rx, ry, self.desert_id, self.forest_id);
        }
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
            description: "将世界划分为不同的环境区域（海洋、森林、丛林、雪原、沙漠）".to_string(),
            steps: vec![
                StepMeta {
                    name: "森林初始化".to_string(),
                    description: "将全世界初始化为森林环境".to_string(),
                    doc_url: None,
                },
                StepMeta {
                    name: "海洋边界".to_string(),
                    description: "将世界最左/最右两侧判定为海洋".to_string(),
                    doc_url: None,
                },
                StepMeta {
                    name: "出生点森林".to_string(),
                    description: "在世界中心设定出生点森林区域".to_string(),
                    doc_url: None,
                },
                StepMeta {
                    name: "森林扩展".to_string(),
                    description: "在出生点森林附近随机扩展森林区域".to_string(),
                    doc_url: None,
                },
                StepMeta {
                    name: "丛林生成".to_string(),
                    description: "在世界一侧生成椭圆形丛林环境".to_string(),
                    doc_url: None,
                },
                StepMeta {
                    name: "雪原生成".to_string(),
                    description: "在世界另一侧生成梯形雪原环境".to_string(),
                    doc_url: None,
                },
                StepMeta {
                    name: "沙漠判定".to_string(),
                    description: "在剩余森林区域中随机生成椭圆形沙漠".to_string(),
                    doc_url: None,
                },
            ],
            params: vec![
                ParamDef {
                    key: "ocean_ratio".to_string(),
                    name: "海洋宽度比例".to_string(),
                    description: "每侧海洋占世界宽度的比例".to_string(),
                    param_type: ParamType::Float { min: 0.01, max: 0.30 },
                    default: serde_json::json!(0.10),
                },
                ParamDef {
                    key: "spawn_width_ratio".to_string(),
                    name: "出生点森林半宽".to_string(),
                    description: "出生点森林的半宽占世界宽度比例".to_string(),
                    param_type: ParamType::Float { min: 0.01, max: 0.20 },
                    default: serde_json::json!(0.05),
                },
                ParamDef {
                    key: "expand_min_ratio".to_string(),
                    name: "森林扩展最小比例".to_string(),
                    description: "森林扩展的最小宽度比例".to_string(),
                    param_type: ParamType::Float { min: 0.01, max: 0.30 },
                    default: serde_json::json!(0.08),
                },
                ParamDef {
                    key: "expand_max_ratio".to_string(),
                    name: "森林扩展最大比例".to_string(),
                    description: "森林扩展的最大宽度比例".to_string(),
                    param_type: ParamType::Float { min: 0.01, max: 0.40 },
                    default: serde_json::json!(0.15),
                },
                ParamDef {
                    key: "jungle_width_min".to_string(),
                    name: "丛林宽度最小比例".to_string(),
                    description: "丛林椭圆宽度的最小比例".to_string(),
                    param_type: ParamType::Float { min: 0.05, max: 0.30 },
                    default: serde_json::json!(0.12),
                },
                ParamDef {
                    key: "jungle_width_max".to_string(),
                    name: "丛林宽度最大比例".to_string(),
                    description: "丛林椭圆宽度的最大比例".to_string(),
                    param_type: ParamType::Float { min: 0.05, max: 0.40 },
                    default: serde_json::json!(0.18),
                },
                ParamDef {
                    key: "snow_top_ratio".to_string(),
                    name: "雪原顶部宽度比例".to_string(),
                    description: "雪原梯形顶部宽度比例".to_string(),
                    param_type: ParamType::Float { min: 0.02, max: 0.25 },
                    default: serde_json::json!(0.08),
                },
                ParamDef {
                    key: "snow_bot_ratio".to_string(),
                    name: "雪原底部宽度比例".to_string(),
                    description: "雪原梯形底部宽度比例".to_string(),
                    param_type: ParamType::Float { min: 0.05, max: 0.35 },
                    default: serde_json::json!(0.16),
                },
                ParamDef {
                    key: "desert_min_count".to_string(),
                    name: "沙漠最少数量".to_string(),
                    description: "随机生成的沙漠最少数量".to_string(),
                    param_type: ParamType::Int { min: 0, max: 10 },
                    default: serde_json::json!(1),
                },
                ParamDef {
                    key: "desert_max_count".to_string(),
                    name: "沙漠最多数量".to_string(),
                    description: "随机生成的沙漠最多数量".to_string(),
                    param_type: ParamType::Int { min: 0, max: 10 },
                    default: serde_json::json!(3),
                },
                ParamDef {
                    key: "desert_width_min".to_string(),
                    name: "沙漠宽度最小比例".to_string(),
                    description: "沙漠椭圆宽度的最小比例".to_string(),
                    param_type: ParamType::Float { min: 0.01, max: 0.20 },
                    default: serde_json::json!(0.05),
                },
                ParamDef {
                    key: "desert_width_max".to_string(),
                    name: "沙漠宽度最大比例".to_string(),
                    description: "沙漠椭圆宽度的最大比例".to_string(),
                    param_type: ParamType::Float { min: 0.02, max: 0.30 },
                    default: serde_json::json!(0.10),
                },
            ],
        }
    }

    fn execute(&mut self, step_index: usize, ctx: &mut RuntimeContext) -> Result<(), String> {
        match step_index {
            0 => self.step_forest_init(ctx),
            1 => self.step_ocean_border(ctx),
            2 => self.step_spawn_forest(ctx),
            3 => self.step_forest_expand(ctx),
            4 => self.step_jungle(ctx),
            5 => self.step_snow(ctx),
            6 => self.step_desert(ctx),
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
        self.jungle_on_left = None;
    }
}
