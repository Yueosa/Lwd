pub mod biome_division;
pub mod pipeline;
pub mod reset;
pub mod step;

use crate::core::biome::BiomeDefinition;

pub use pipeline::GenerationPipeline;
pub use step::{GenerationContext, GenerationState, GenerationStep, StepInfo, StepStatus};

use self::biome_division::BiomeDivisionStep;
use self::reset::ResetStep;

/// Build the default generation pipeline.
///
/// **To add a new step**, just append
/// `pipeline.register(Box::new(YourStep))` below.
///
/// Each step automatically gets:
/// - Deterministic seeded RNG
/// - Undo / replay support
/// - Automatic texture refresh after execution
pub fn build_default_pipeline(seed: u64, biome_definitions: Vec<BiomeDefinition>) -> GenerationPipeline {
    let mut pipeline = GenerationPipeline::new(seed);

    // ── registered steps (order matters) ────────────────────
    pipeline.register(Box::new(ResetStep));
    pipeline.register(Box::new(BiomeDivisionStep::new(biome_definitions)));
    // pipeline.register(Box::new(terrain::TerrainStep::new()));
    // pipeline.register(Box::new(dunes::DunesStep::new()));
    // ...在这里继续添加步骤
    // ────────────────────────────────────────────────────────

    pipeline
}
