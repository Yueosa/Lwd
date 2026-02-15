use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::core::block::BlockDefinition;
use crate::core::world::{World, WorldProfile};

use super::step::{
    GenerationContext, GenerationPhase, GenerationState, PhaseInfo, StepStatus, SubStepInfo,
};

/// Manages an ordered list of generation phases, each containing sub-steps.
///
/// Supports forward/backward stepping at both phase and sub-step granularity.
/// Each sub-step gets its own RNG derived from a master seed, so
/// replaying from scratch always reproduces the same world.
pub struct GenerationPipeline {
    phases: Vec<GenerationPhase>,
    /// Master seed — each sub-step derives a sub-seed from this.
    seed: u64,
    /// Shared state across steps (e.g., BiomeMap).
    state: GenerationState,
    /// Current execution position: (phase_index, sub_step_index within that phase).
    /// Points to the *next* sub-step to execute.
    /// When all done: current_phase == phases.len()
    current_phase: usize,
    current_sub: usize,
}

impl GenerationPipeline {
    pub fn new(seed: u64) -> Self {
        Self {
            phases: Vec::new(),
            seed,
            state: GenerationState::new(),
            current_phase: 0,
            current_sub: 0,
        }
    }

    /// Append a phase to the pipeline.
    pub fn register_phase(&mut self, phase: GenerationPhase) {
        self.phases.push(phase);
    }

    // ── accessors ───────────────────────────────────────────

    /// Total number of sub-steps across all phases.
    pub fn total_sub_steps(&self) -> usize {
        self.phases.iter().map(|p| p.sub_steps.len()).sum()
    }

    /// Number of sub-steps already executed.
    pub fn executed_sub_steps(&self) -> usize {
        let full_phases: usize = self.phases[..self.current_phase]
            .iter()
            .map(|p| p.sub_steps.len())
            .sum();
        full_phases + self.current_sub
    }

    pub fn is_complete(&self) -> bool {
        self.current_phase >= self.phases.len()
    }

    /// 当前步骤的显示ID (如 "1.0", "1.1", "2.0")
    pub fn current_step_id(&self) -> Option<String> {
        if self.current_phase >= self.phases.len() {
            return None;
        }
        let phase = &self.phases[self.current_phase];
        if self.current_sub < phase.sub_steps.len() {
            Some(phase.sub_steps[self.current_sub].meta.id.clone())
        } else {
            None
        }
    }

    /// Name of the step that was last executed, or `None`.
    pub fn last_executed_name(&self) -> Option<String> {
        // Find position of the step just before current
        if self.current_phase == 0 && self.current_sub == 0 {
            return None;
        }

        let (p, s) = self.prev_position()?;
        let phase = &self.phases[p];
        Some(format!(
            "{} - {}",
            phase.name, phase.sub_steps[s].meta.name
        ))
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn set_seed(&mut self, seed: u64) {
        self.seed = seed;
    }

    /// 获取当前生成状态的引用 (用于 UI 访问 BiomeMap)
    pub fn state(&self) -> &GenerationState {
        &self.state
    }

    /// 当前所在 phase index（用于 UI 判断"打开当前步骤配置"）
    pub fn current_phase_index(&self) -> usize {
        self.current_phase
    }

    /// 当前所在 sub-step index
    pub fn current_sub_index(&self) -> usize {
        self.current_sub
    }

    /// 获取当前子步骤的可变引用，用于打开配置面板
    pub fn current_sub_step_mut(
        &mut self,
    ) -> Option<&mut super::step::PhaseSubStep> {
        if self.current_phase >= self.phases.len() {
            return None;
        }
        let phase = &mut self.phases[self.current_phase];
        // 显示的是"当前"子步骤，即已执行的最后一步或下一步
        // 如果 current_sub > 0，显示上一步的配置更合理
        if self.current_sub > 0 && self.current_sub <= phase.sub_steps.len() {
            Some(&mut phase.sub_steps[self.current_sub - 1])
        } else if !phase.sub_steps.is_empty() {
            Some(&mut phase.sub_steps[0])
        } else {
            None
        }
    }

    // ── stepping ────────────────────────────────────────────

    /// Execute the next pending sub-step (小步 +0.1).
    pub fn step_forward_sub(
        &mut self,
        world: &mut World,
        profile: &WorldProfile,
        blocks: &[BlockDefinition],
    ) -> Result<bool, String> {
        if self.is_complete() {
            return Ok(false);
        }

        let flat_index = self.executed_sub_steps();
        let step_seed = derive_step_seed(self.seed, flat_index);
        let mut rng = StdRng::seed_from_u64(step_seed);

        let phase = &self.phases[self.current_phase];
        let sub = &phase.sub_steps[self.current_sub];

        let mut ctx = GenerationContext {
            world,
            profile,
            blocks,
            rng: &mut rng,
            state: &mut self.state,
        };

        sub.step
            .execute(&mut ctx)
            .map_err(|e| format!("{} - {}: {e}", phase.name, sub.meta.name))?;

        // Advance position
        self.current_sub += 1;
        if self.current_sub >= phase.sub_steps.len() {
            self.current_phase += 1;
            self.current_sub = 0;
        }

        Ok(true)
    }

    /// Execute all remaining sub-steps in the current phase (大步 +1.0).
    pub fn step_forward_phase(
        &mut self,
        world: &mut World,
        profile: &WorldProfile,
        blocks: &[BlockDefinition],
    ) -> Result<bool, String> {
        if self.is_complete() {
            return Ok(false);
        }

        let target_phase = self.current_phase;
        let mut any_ran = false;

        // Run until we leave this phase or complete everything
        while self.current_phase == target_phase && !self.is_complete() {
            self.step_forward_sub(world, profile, blocks)?;
            any_ran = true;
        }

        Ok(any_ran)
    }

    /// Undo one sub-step by replaying from scratch (小步 -0.1).
    pub fn step_backward_sub(
        &mut self,
        world: &mut World,
        profile: &WorldProfile,
        blocks: &[BlockDefinition],
    ) -> Result<bool, String> {
        let executed = self.executed_sub_steps();
        if executed == 0 {
            return Ok(false);
        }
        self.replay_to_flat(executed - 1, world, profile, blocks)?;
        Ok(true)
    }

    /// Undo to the start of the current phase (大步 -1.0).
    pub fn step_backward_phase(
        &mut self,
        world: &mut World,
        profile: &WorldProfile,
        blocks: &[BlockDefinition],
    ) -> Result<bool, String> {
        let executed = self.executed_sub_steps();
        if executed == 0 {
            return Ok(false);
        }

        // If we're at the beginning of a phase, go to the beginning of the previous one
        let target = if self.current_sub == 0 && self.current_phase > 0 {
            // Go to start of previous phase
            self.phases[..self.current_phase - 1]
                .iter()
                .map(|p| p.sub_steps.len())
                .sum()
        } else {
            // Go to start of current phase
            self.phases[..self.current_phase]
                .iter()
                .map(|p| p.sub_steps.len())
                .sum()
        };

        self.replay_to_flat(target, world, profile, blocks)?;
        Ok(true)
    }

    /// Reset to step 0: clear the world to air.
    pub fn reset_all(&mut self, world: &mut World) {
        *world = World::new_air(world.width, world.height);
        self.current_phase = 0;
        self.current_sub = 0;
        self.state = GenerationState::new();
    }

    /// Run every remaining sub-step at once.
    pub fn run_all(
        &mut self,
        world: &mut World,
        profile: &WorldProfile,
        blocks: &[BlockDefinition],
    ) -> Result<(), String> {
        while !self.is_complete() {
            self.step_forward_sub(world, profile, blocks)?;
        }
        Ok(())
    }

    // ── UI info ─────────────────────────────────────────────

    /// Build a list of phase info for the control panel.
    pub fn phase_info_list(&self) -> Vec<PhaseInfo> {
        let executed = self.executed_sub_steps();
        let mut flat = 0usize;

        self.phases
            .iter()
            .enumerate()
            .map(|(_pi, phase)| {
                let phase_start = flat;
                let sub_infos: Vec<SubStepInfo> = phase
                    .sub_steps
                    .iter()
                    .enumerate()
                    .map(|(_si, sub)| {
                        let status = if flat < executed {
                            StepStatus::Completed
                        } else if flat == executed {
                            StepStatus::Current
                        } else {
                            StepStatus::Pending
                        };
                        flat += 1;
                        SubStepInfo {
                            id: sub.meta.id.clone(),
                            name: sub.meta.name.clone(),
                            description: sub.meta.description.clone(),
                            doc_url: sub.meta.doc_url.clone(),
                            status,
                            has_config: sub.step.has_config(),
                        }
                    })
                    .collect();

                let phase_end = flat;
                let phase_status = if phase_end <= executed {
                    StepStatus::Completed
                } else if phase_start >= executed {
                    StepStatus::Pending
                } else {
                    StepStatus::Current
                };

                PhaseInfo {
                    phase_id: phase.phase_id,
                    name: phase.name.clone(),
                    description: phase.description.clone(),
                    sub_steps: sub_infos,
                    status: phase_status,
                }
            })
            .collect()
    }

    // ── internal ────────────────────────────────────────────

    /// Convert flat index back to (phase, sub) position.
    fn flat_to_position(&self, flat: usize) -> (usize, usize) {
        let mut remaining = flat;
        for (pi, phase) in self.phases.iter().enumerate() {
            if remaining < phase.sub_steps.len() {
                return (pi, remaining);
            }
            remaining -= phase.sub_steps.len();
        }
        // Past the end
        (self.phases.len(), 0)
    }

    /// Get the (phase, sub) position of the step before current.
    fn prev_position(&self) -> Option<(usize, usize)> {
        let executed = self.executed_sub_steps();
        if executed == 0 {
            return None;
        }
        Some(self.flat_to_position(executed - 1))
    }

    fn replay_to_flat(
        &mut self,
        target_flat: usize,
        world: &mut World,
        profile: &WorldProfile,
        blocks: &[BlockDefinition],
    ) -> Result<(), String> {
        *world = World::new_air(world.width, world.height);
        self.state = GenerationState::new();
        self.current_phase = 0;
        self.current_sub = 0;

        for _ in 0..target_flat {
            self.step_forward_sub(world, profile, blocks)?;
        }

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
