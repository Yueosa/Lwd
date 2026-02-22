//! # 环境判定算法模块
//!
//! 这是一个独立的算法模块，通过 [`PhaseAlgorithm`] trait 向引擎声明自身。
//! 引擎不感知此模块内部逻辑，只通过 `meta()` / `execute()` / `get_params()` / `set_params()` 交互。

use crate::core::biome::{BiomeDefinition, BiomeId};
use crate::generation::algorithm::{
    ParamDef, ParamType, PhaseAlgorithm, PhaseMeta, RuntimeContext, StepMeta,
};

// 模块声明
mod params;
mod space_hell;
mod ocean;
mod forest;
mod jungle;
mod snow;
mod desert;
mod crimson;
mod forest_fill;
mod stone_fill;

// 导出参数
pub use params::BiomeDivisionParams;

// ═══════════════════════════════════════════════════════════
// 辅助函数
// ═══════════════════════════════════════════════════════════

/// 根据 key 查找 biome ID
fn biome_id_by_key(defs: &[BiomeDefinition], key: &str) -> Option<BiomeId> {
    defs.iter().find(|b| b.key == key).map(|b| b.id)
}

// ═══════════════════════════════════════════════════════════
// 算法模块
// ═══════════════════════════════════════════════════════════

pub struct BiomeDivisionAlgorithm {
    /// 环境定义列表（用于运行时动态查找）
    biome_definitions: Vec<BiomeDefinition>,
    /// 可调参数
    pub params: BiomeDivisionParams,
}

impl BiomeDivisionAlgorithm {
    pub fn new(biome_definitions: &[BiomeDefinition]) -> Self {
        Self {
            biome_definitions: biome_definitions.to_vec(),
            params: BiomeDivisionParams::default(),
        }
    }
    
    /// 根据 key 查找 biome ID
    pub fn get_biome_id(&self, key: &str) -> Option<BiomeId> {
        biome_id_by_key(&self.biome_definitions, key)
    }
    
    /// 根据 biome ID 获取 overlay_color
    pub fn biome_color(&self, id: BiomeId) -> [u8; 4] {
        self.biome_definitions.iter()
            .find(|b| b.id == id)
            .map(|b| b.overlay_color)
            .unwrap_or([128, 128, 128, 120])
    }

    // ── 各子步骤实现（调用对应模块） ────────────────────

    fn step_space_hell(&self, ctx: &mut RuntimeContext) -> Result<(), String> {
        space_hell::execute(self, ctx)
    }

    fn step_ocean(&self, ctx: &mut RuntimeContext) -> Result<(), String> {
        ocean::execute(self, ctx)
    }

    fn step_forest(&self, ctx: &mut RuntimeContext) -> Result<(), String> {
        forest::execute(self, ctx)
    }

    fn step_jungle(&self, ctx: &mut RuntimeContext) -> Result<(), String> {
        jungle::execute(self, ctx)
    }

    fn step_snow(&self, ctx: &mut RuntimeContext) -> Result<(), String> {
        snow::execute(self, ctx)
    }

    fn step_desert(&self, ctx: &mut RuntimeContext) -> Result<(), String> {
        desert::execute(self, ctx)
    }

    fn step_crimson(&self, ctx: &mut RuntimeContext) -> Result<(), String> {
        crimson::execute(self, ctx)
    }

    fn step_forest_fill(&self, ctx: &mut RuntimeContext) -> Result<(), String> {
        forest_fill::execute(self, ctx)
    }

    fn step_stone_fill(&self, ctx: &mut RuntimeContext) -> Result<(), String> {
        stone_fill::execute(self, ctx)
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
                },
                ParamDef {
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
