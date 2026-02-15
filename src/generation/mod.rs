pub mod algorithm;
pub mod pipeline;
pub mod snapshot;

use crate::algorithms::biome_division::BiomeDivisionAlgorithm;
use crate::core::biome::BiomeDefinition;

pub use algorithm::{PhaseAlgorithm, PhaseMeta, StepMeta, ParamDef, ParamType, RuntimeContext};
pub use pipeline::{GenerationPipeline, PhaseInfo, SubStepInfo, StepStatus};
pub use snapshot::{WorldSnapshot, export_png};

/// 构建默认流水线，注册所有算法模块。
///
/// 新增算法模块只需：
/// 1. 在 `src/algorithms/` 下创建实现 `PhaseAlgorithm` 的模块
/// 2. 在此函数中 `pipeline.register(Box::new(YourAlgorithm::new(...)))`
///
/// 引擎会自动：读取 meta() → 构建 UI 步骤列表 → 按顺序执行。
pub fn build_pipeline(
    seed: u64,
    biome_definitions: Vec<BiomeDefinition>,
) -> GenerationPipeline {
    let mut pipeline = GenerationPipeline::new(seed, biome_definitions.clone());

    // ── Phase 1: 环境判定 ──
    pipeline.register(Box::new(BiomeDivisionAlgorithm::new(&biome_definitions)));

    // ── Phase 2+: 未来在此注册更多算法模块 ──

    pipeline
}
