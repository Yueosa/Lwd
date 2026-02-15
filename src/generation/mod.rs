pub mod biome_division;
pub mod pipeline;
pub mod step;

use crate::config::steps::{load_steps_config, PhaseMeta};
use crate::core::biome::BiomeDefinition;

pub use pipeline::GenerationPipeline;
pub use step::{
    GenerationContext, GenerationPhase, GenerationStep,
    PhaseInfo, PhaseSubStep, StepStatus,
};

use self::biome_division::{BiomeOceanBorderStep, BiomeInlandDivisionStep};

/// Build the default generation pipeline.
///
/// Reads phase/step metadata from `assets/steps.json`,
/// then wires up the concrete step implementations.
///
/// **To add a new step:**
/// 1. Add metadata to `assets/steps.json`
/// 2. Implement `GenerationStep` trait
/// 3. Wire it up in the `match` below
pub fn build_default_pipeline(
    seed: u64,
    biome_definitions: Vec<BiomeDefinition>,
) -> GenerationPipeline {
    let steps_config = load_steps_config().expect("steps.json 加载失败");
    let mut pipeline = GenerationPipeline::new(seed);

    for phase_meta in &steps_config.phases {
        let phase = build_phase(phase_meta, &biome_definitions);
        pipeline.register_phase(phase);
    }

    pipeline
}

/// 根据 phase 元数据构建 GenerationPhase，匹配具体的步骤实现
fn build_phase(
    meta: &PhaseMeta,
    biome_definitions: &[BiomeDefinition],
) -> GenerationPhase {
    let mut sub_steps = Vec::new();

    for sub_meta in &meta.sub_steps {
        let step: Box<dyn GenerationStep> = match sub_meta.id.as_str() {
            // ── Phase 1: 环境划分 ──
            "1.0" => Box::new(BiomeOceanBorderStep::new(biome_definitions.to_vec())),
            "1.1" => Box::new(BiomeInlandDivisionStep::new(biome_definitions.to_vec())),
            // ── Phase 2+: 占位（尚未实现）──
            other => Box::new(PlaceholderStep(other.to_string())),
        };

        sub_steps.push(PhaseSubStep {
            meta: sub_meta.clone(),
            step,
        });
    }

    GenerationPhase {
        phase_id: meta.id,
        name: meta.name.clone(),
        description: meta.description.clone(),
        sub_steps,
    }
}

/// 占位步骤 — 尚未实现的步骤，execute 时直接 Ok
struct PlaceholderStep(String);

impl GenerationStep for PlaceholderStep {
    fn execute(&self, _ctx: &mut GenerationContext) -> Result<(), String> {
        eprintln!("[跳过] 步骤 {} 尚未实现", self.0);
        Ok(())
    }
}
