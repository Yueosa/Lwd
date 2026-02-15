use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::core::block::BlockDefinition;
use crate::core::world::{World, WorldProfile};

use super::step::{GenerationContext, GenerationStep, StepInfo, StepStatus};

/// Manages an ordered list of generation steps.
///
/// Supports forward/backward stepping with deterministic replay.
/// Each step gets its own RNG derived from a master seed, so
/// replaying from scratch always reproduces the same world.
pub struct GenerationPipeline {
    steps: Vec<Box<dyn GenerationStep>>,
    /// Number of steps already executed (0 = none).
    executed: usize,
    /// Master seed — each step derives a sub-seed from this.
    seed: u64,
}

impl GenerationPipeline {
    pub fn new(seed: u64) -> Self {
        Self {
            steps: Vec::new(),
            executed: 0,
            seed,
        }
    }

    /// Append a step to the pipeline.
    pub fn register(&mut self, step: Box<dyn GenerationStep>) {
        self.steps.push(step);
    }

    // ── accessors ───────────────────────────────────────────

    pub fn total_steps(&self) -> usize {
        self.steps.len()
    }

    pub fn executed_count(&self) -> usize {
        self.executed
    }

    pub fn is_complete(&self) -> bool {
        self.executed >= self.steps.len()
    }

    /// Name of the step that was last executed, or `None`.
    pub fn last_executed_name(&self) -> Option<&str> {
        if self.executed > 0 {
            Some(self.steps[self.executed - 1].name())
        } else {
            None
        }
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn set_seed(&mut self, seed: u64) {
        self.seed = seed;
    }

    // ── stepping ────────────────────────────────────────────

    /// Execute the next pending step.
    ///
    /// Returns `Ok(true)` if a step ran, `Ok(false)` if all
    /// steps are already complete, or `Err` on failure.
    pub fn step_forward(
        &mut self,
        world: &mut World,
        profile: &WorldProfile,
        blocks: &[BlockDefinition],
    ) -> Result<bool, String> {
        if self.executed >= self.steps.len() {
            return Ok(false);
        }

        let step_seed = derive_step_seed(self.seed, self.executed);
        let mut rng = StdRng::seed_from_u64(step_seed);
        let mut ctx = GenerationContext {
            world,
            profile,
            blocks,
            rng: &mut rng,
        };

        self.steps[self.executed]
            .execute(&mut ctx)
            .map_err(|e| format!("{}: {e}", self.steps[self.executed].name()))?;

        self.executed += 1;
        Ok(true)
    }

    /// Undo the last step by replaying 0 .. executed-1 from scratch.
    ///
    /// Returns `Ok(false)` if already at step 0.
    pub fn step_backward(
        &mut self,
        world: &mut World,
        profile: &WorldProfile,
        blocks: &[BlockDefinition],
    ) -> Result<bool, String> {
        if self.executed == 0 {
            return Ok(false);
        }
        let target = self.executed - 1;
        self.replay_to(target, world, profile, blocks)?;
        Ok(true)
    }

    /// Reset to step 0: clear the world to air.
    pub fn reset_all(&mut self, world: &mut World) {
        *world = World::new_air(world.width, world.height);
        self.executed = 0;
    }

    /// Run every remaining step at once.
    pub fn run_all(
        &mut self,
        world: &mut World,
        profile: &WorldProfile,
        blocks: &[BlockDefinition],
    ) -> Result<(), String> {
        while self.executed < self.steps.len() {
            self.step_forward(world, profile, blocks)?;
        }
        Ok(())
    }

    // ── UI info ─────────────────────────────────────────────

    /// Build a list of step info for the control panel.
    pub fn step_info_list(&self) -> Vec<StepInfo> {
        self.steps
            .iter()
            .enumerate()
            .map(|(i, step)| StepInfo {
                name: step.name().to_string(),
                description: step.description().to_string(),
                status: if i < self.executed {
                    StepStatus::Completed
                } else if i == self.executed {
                    StepStatus::Current
                } else {
                    StepStatus::Pending
                },
            })
            .collect()
    }

    // ── internal ────────────────────────────────────────────

    fn replay_to(
        &mut self,
        target: usize,
        world: &mut World,
        profile: &WorldProfile,
        blocks: &[BlockDefinition],
    ) -> Result<(), String> {
        *world = World::new_air(world.width, world.height);

        for i in 0..target {
            let step_seed = derive_step_seed(self.seed, i);
            let mut rng = StdRng::seed_from_u64(step_seed);
            let mut ctx = GenerationContext {
                world,
                profile,
                blocks,
                rng: &mut rng,
            };
            self.steps[i]
                .execute(&mut ctx)
                .map_err(|e| format!("{}: {e}", self.steps[i].name()))?;
        }

        self.executed = target;
        Ok(())
    }
}

/// Deterministic per-step seed derived from the master seed.
fn derive_step_seed(master: u64, step_index: usize) -> u64 {
    master
        .wrapping_add(step_index as u64)
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407)
}
