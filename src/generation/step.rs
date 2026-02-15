use rand::rngs::StdRng;

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
///
/// * `world`   — mutable reference to the tile matrix
/// * `profile` — world size + layer info (read-only)
/// * `blocks`  — all block definitions (read-only)
/// * `rng`     — seeded RNG; use this for **all** randomness
///               to guarantee deterministic replay
/// * `state`   — shared state across steps (e.g., BiomeMap)
pub struct GenerationContext<'a> {
    pub world: &'a mut World,
    pub profile: &'a WorldProfile,
    pub blocks: &'a [BlockDefinition],
    pub rng: &'a mut StdRng,
    pub state: &'a mut GenerationState,
}

// ── Step trait ───────────────────────────────────────────────

/// Trait that every generation step must implement.
///
/// # How to add a new step
///
/// 1. Create a file in `src/generation/`, e.g. `terrain.rs`
/// 2. Implement this trait on a struct
/// 3. Register it in [`super::build_default_pipeline()`]
/// 4. Done — the pipeline handles execution, undo, and rendering.
pub trait GenerationStep {
    /// Display name (shown in the step list)
    fn name(&self) -> &str;

    /// Short description of what this step does
    fn description(&self) -> &str;

    /// Execute the step, modifying `ctx.world` in place.
    ///
    /// Use `ctx.rng` for any randomness so that backward/replay
    /// produces identical results.
    ///
    /// Return `Err(message)` to signal a failure.
    fn execute(&self, ctx: &mut GenerationContext) -> Result<(), String>;
}

// ── UI helpers ──────────────────────────────────────────────

/// Display status in the step list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStatus {
    Completed,
    Current,
    Pending,
}

/// Snapshot of one step for the control panel.
#[derive(Debug, Clone)]
pub struct StepInfo {
    pub name: String,
    pub description: String,
    pub status: StepStatus,
}
