use rand::rngs::StdRng;

use crate::config::steps::SubStepMeta;
use crate::core::biome::BiomeMap;
use crate::core::block::BlockDefinition;
use crate::core::world::{World, WorldProfile};

// ── State ────────────────────────────────────────────────

/// 生成过程中的共享状态（在步骤间传递）
#[derive(Debug, Clone)]
pub struct GenerationState {
    /// 环境地图（水平划分）
    pub biome_map: Option<BiomeMap>,
}

impl GenerationState {
    pub fn new() -> Self {
        Self { biome_map: None }
    }
}

impl Default for GenerationState {
    fn default() -> Self {
        Self::new()
    }
}

// ── Context ─────────────────────────────────────────────────

/// Everything a generation step needs to do its work.
pub struct GenerationContext<'a> {
    pub world: &'a mut World,
    pub profile: &'a WorldProfile,
    pub blocks: &'a [BlockDefinition],
    pub rng: &'a mut StdRng,
    pub state: &'a mut GenerationState,
}

// ── Step trait ───────────────────────────────────────────────

/// Trait that every generation sub-step must implement.
///
/// # How to add a new step
///
/// 1. Create a file in `src/generation/`, e.g. `terrain.rs`
/// 2. Implement this trait on a struct
/// 3. Register it in a Phase via [`super::build_default_pipeline()`]
/// 4. Add metadata to `assets/steps.json`
pub trait GenerationStep {
    /// Execute the step, modifying `ctx.world` in place.
    fn execute(&self, ctx: &mut GenerationContext) -> Result<(), String>;

    /// 返回 true 表示该步骤有可配置的算法参数
    fn has_config(&self) -> bool {
        false
    }

    /// 绘制算法参数配置 UI（egui）
    fn show_config_ui(&mut self, _ui: &mut egui::Ui) {
        // 默认无配置
    }
}

// ── Phase ───────────────────────────────────────────────────

/// 一个大阶段（Phase），包含多个子步骤。
///
/// 例如：Phase 1 "环境划分" 包含 1.0 "海洋边界" + 1.1 "内陆划分"
pub struct GenerationPhase {
    /// Phase 编号 (1, 2, 3...)
    pub phase_id: u32,
    /// Phase 名称
    pub name: String,
    /// Phase 描述
    pub description: String,
    /// 子步骤列表
    pub sub_steps: Vec<PhaseSubStep>,
}

/// Phase 内的一个子步骤 = 元数据 + 执行逻辑
pub struct PhaseSubStep {
    /// 子步骤元数据 (id, name, description, doc_url)
    pub meta: SubStepMeta,
    /// 生成逻辑实现
    pub step: Box<dyn GenerationStep>,
}

// ── UI helpers ──────────────────────────────────────────────

/// Display status in the step list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStatus {
    Completed,
    Current,
    Pending,
}

/// Snapshot of one phase for the control panel.
#[derive(Debug, Clone)]
pub struct PhaseInfo {
    pub phase_id: u32,
    pub name: String,
    pub description: String,
    pub sub_steps: Vec<SubStepInfo>,
    pub status: StepStatus,
}

/// Snapshot of one sub-step for the control panel.
#[derive(Debug, Clone)]
pub struct SubStepInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub doc_url: Option<String>,
    pub status: StepStatus,
    pub has_config: bool,
}
