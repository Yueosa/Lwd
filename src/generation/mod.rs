pub mod pipeline;
pub mod reset;
pub mod step;

pub use pipeline::GenerationPipeline;
pub use step::{GenerationContext, GenerationStep, StepInfo, StepStatus};

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
pub fn build_default_pipeline(seed: u64) -> GenerationPipeline {
    let mut pipeline = GenerationPipeline::new(seed);

    // ── registered steps (order matters) ────────────────────
    pipeline.register(Box::new(ResetStep));
    // pipeline.register(Box::new(terrain::TerrainStep::new()));
    // pipeline.register(Box::new(dunes::DunesStep::new()));
    // ...在这里继续添加步骤
    // ────────────────────────────────────────────────────────

    pipeline
}
