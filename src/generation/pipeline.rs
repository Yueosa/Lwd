//! # 生成流水线
//!
//! 管理一组 [`PhaseAlgorithm`] 模块的执行顺序，支持子步骤/阶段粒度的前进/后退。
//! 每个子步骤使用从主种子派生的确定性 RNG，因此从头回放总能复现相同的世界。

use std::any::Any;
use std::collections::HashMap;
use std::time::Instant;

use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::core::biome::{BiomeDefinition, BiomeMap};
use crate::core::block::BlockDefinition;
use crate::core::geometry::ShapeRecord;
use crate::core::world::{World, WorldProfile};

use super::algorithm::{PhaseAlgorithm, RuntimeContext};
use super::optimizer::PerfProfiler;

// ═══════════════════════════════════════════════════════════
// UI 信息快照（只读，供控制面板展示）
// ═══════════════════════════════════════════════════════════

/// 步骤状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStatus {
    Completed,
    Current,
    Pending,
}

/// 单个子步骤的 UI 快照
#[derive(Debug, Clone)]
pub struct SubStepInfo {
    /// 显示用 ID（如 "1.0", "1.1"）
    pub display_id: String,
    pub name: String,
    pub description: String,
    pub doc_url: Option<String>,
    pub status: StepStatus,
}

/// 单个阶段的 UI 快照
#[derive(Debug, Clone)]
pub struct PhaseInfo {
    /// 阶段序号（从 1 开始显示）
    pub display_index: u32,
    /// 算法模块 ID
    pub algorithm_id: String,
    pub name: String,
    pub description: String,
    pub has_params: bool,
    pub sub_steps: Vec<SubStepInfo>,
    pub status: StepStatus,
}

// ═══════════════════════════════════════════════════════════
// 流水线
// ═══════════════════════════════════════════════════════════

pub struct GenerationPipeline {
    /// 已注册的算法模块（有序）
    algorithms: Vec<Box<dyn PhaseAlgorithm>>,
    /// 每个算法模块的子步骤数缓存（避免每帧调用 meta()）
    step_counts: Vec<usize>,
    /// 总子步骤数缓存
    total_steps_cache: usize,
    /// 主种子
    seed: u64,
    /// 共享的环境地图状态
    biome_map: Option<BiomeMap>,
    /// 通用共享状态容器（跨算法/跨步骤）
    shared_state: HashMap<String, Box<dyn Any>>,
    /// 环境定义（传给 RuntimeContext）
    biome_definitions: Vec<BiomeDefinition>,
    /// 当前执行位置：指向下一个要执行的子步骤
    current_phase: usize,
    current_sub: usize,
    /// 每个子步骤的形状记录（key = flat_index）
    shape_logs: HashMap<usize, Vec<ShapeRecord>>,
    /// phase_info 缓存, 仅在步骤变化时重建
    cached_phase_info: Vec<PhaseInfo>,
    cached_phase_info_executed: usize,
    phase_info_dirty: bool,
    /// 性能分析器
    profiler: PerfProfiler,
}

impl GenerationPipeline {
    pub fn new(seed: u64, biome_definitions: Vec<BiomeDefinition>) -> Self {
        Self {
            algorithms: Vec::new(),
            step_counts: Vec::new(),
            total_steps_cache: 0,
            seed,
            biome_map: None,
            shared_state: HashMap::new(),
            biome_definitions,
            current_phase: 0,
            current_sub: 0,
            shape_logs: HashMap::new(),
            cached_phase_info: Vec::new(),
            cached_phase_info_executed: usize::MAX,
            phase_info_dirty: true,
            profiler: PerfProfiler::new(),
        }
    }

    /// 注册一个算法模块
    pub fn register(&mut self, algorithm: Box<dyn PhaseAlgorithm>) {
        let count = algorithm.meta().steps.len();
        self.total_steps_cache += count;
        self.step_counts.push(count);
        self.algorithms.push(algorithm);
        self.phase_info_dirty = true;
    }

    // ── 访问器 ──────────────────────────────────────────────

    /// 总子步骤数（O(1) 缓存）
    pub fn total_sub_steps(&self) -> usize {
        self.total_steps_cache
    }

    /// 已执行子步骤数（O(1) 使用缓存的 step_counts）
    pub fn executed_sub_steps(&self) -> usize {
        let full: usize = self.step_counts[..self.current_phase].iter().sum();
        full + self.current_sub
    }

    pub fn is_complete(&self) -> bool {
        self.current_phase >= self.algorithms.len()
    }

    /// 当前步骤的显示 ID (如 "1.1", "1.2")
    pub fn current_step_display_id(&self) -> Option<String> {
        if self.current_phase >= self.algorithms.len() {
            return None;
        }
        if self.current_sub < self.step_counts[self.current_phase] {
            let meta = self.algorithms[self.current_phase].meta();
            let display_idx = meta.steps[self.current_sub].display_index;
            Some(format!("{}.{}", self.current_phase + 1, display_idx))
        } else {
            None
        }
    }

    /// 最后执行的步骤名称
    pub fn last_executed_name(&self) -> Option<String> {
        let executed = self.executed_sub_steps();
        if executed == 0 {
            return None;
        }
        let (p, s) = self.flat_to_position(executed - 1);
        let meta = self.algorithms[p].meta();
        Some(format!("{} - {}", meta.name, meta.steps[s].name))
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn set_seed(&mut self, seed: u64) {
        self.seed = seed;
    }

    /// 获取 biome_map 引用（供 UI 渲染 overlay）
    pub fn biome_map(&self) -> Option<&BiomeMap> {
        self.biome_map.as_ref()
    }

    /// 获取指定子步骤的形状记录（flat_index）
    pub fn shape_log(&self, flat_index: usize) -> Option<&[ShapeRecord]> {
        self.shape_logs.get(&flat_index).map(|v| v.as_slice())
    }

    /// 获取最后执行的子步骤的形状记录
    pub fn last_executed_shape_log(&self) -> Option<&[ShapeRecord]> {
        let executed = self.executed_sub_steps();
        if executed == 0 { return None; }
        self.shape_log(executed - 1)
    }

    pub fn current_phase_index(&self) -> usize {
        self.current_phase
    }

    pub fn current_sub_index(&self) -> usize {
        self.current_sub
    }

    /// 获取指定阶段的算法模块的可变引用
    pub fn algorithm_mut(&mut self, phase_index: usize) -> Option<&mut Box<dyn PhaseAlgorithm>> {
        self.algorithms.get_mut(phase_index)
    }

    /// 获取"当前"算法模块（可用于打开配置面板）
    /// 如果已执行了一些步骤，返回最后执行的那个算法模块
    pub fn current_algorithm_mut(&mut self) -> Option<(usize, &mut Box<dyn PhaseAlgorithm>)> {
        if self.algorithms.is_empty() {
            return None;
        }
        let idx = if self.current_sub > 0 || self.current_phase == 0 {
            self.current_phase.min(self.algorithms.len() - 1)
        } else {
            self.current_phase - 1
        };
        Some((idx, &mut self.algorithms[idx]))
    }

    // ── 步进控制 ────────────────────────────────────────────

    /// 小步前进（+0.1）
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
        let step_seed = derive_step_seed(self.seed, flat_index, profile.size.width, profile.size.height);
        let mut rng = StdRng::seed_from_u64(step_seed);
        let mut step_shapes: Vec<ShapeRecord> = Vec::new();

        let step_count = self.step_counts[self.current_phase];

        // 获取步骤名称用于性能记录
        let step_name = {
            let meta = self.algorithms[self.current_phase].meta();
            format!("{} - {}", meta.name, meta.steps[self.current_sub].name)
        };

        let mut ctx = RuntimeContext {
            world,
            profile,
            blocks,
            biomes: &self.biome_definitions,
            rng: &mut rng,
            biome_map: &mut self.biome_map,
            shared: &mut self.shared_state,
            shape_log: &mut step_shapes,
        };

        // 带计时的步骤执行
        let t0 = Instant::now();
        self.algorithms[self.current_phase]
            .execute(self.current_sub, &mut ctx)
            .map_err(|e| {
                let meta = self.algorithms[self.current_phase].meta();
                format!("{}: {e}", meta.name)
            })?;
        let elapsed = t0.elapsed();
        self.profiler.record_step(flat_index, &step_name, elapsed);

        // 保存此步骤的形状记录
        self.shape_logs.insert(flat_index, step_shapes);

        // 推进位置
        self.current_sub += 1;
        if self.current_sub >= step_count {
            self.current_phase += 1;
            self.current_sub = 0;
        }
        self.phase_info_dirty = true;

        Ok(true)
    }

    /// 大步前进（+1.0）
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

        while self.current_phase == target_phase && !self.is_complete() {
            self.step_forward_sub(world, profile, blocks)?;
            any_ran = true;
        }

        Ok(any_ran)
    }

    /// 小步后退（-0.1）
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

    /// 大步后退（-1.0）
    ///
    /// 语义：回到当前 phase 开头。如果已在 phase 开头，则回到前一个 phase 开头。
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

        // 当前 phase 的起始 flat 位置（使用缓存的 step_counts）
        let current_phase_start: usize = self.step_counts[..self.current_phase].iter().sum();

        let target = if executed > current_phase_start {
            // 还没到 phase 开头 → 回到当前 phase 开头
            current_phase_start
        } else if self.current_phase > 0 {
            // 已经在 phase 开头 → 回到前一个 phase 开头
            self.step_counts[..self.current_phase - 1].iter().sum()
        } else {
            0
        };

        self.replay_to_flat(target, world, profile, blocks)?;
        Ok(true)
    }

    /// 重置到第 0 步
    pub fn reset_all(&mut self, world: &mut World) {
        *world = World::new_air(world.width, world.height);
        self.current_phase = 0;
        self.current_sub = 0;
        self.biome_map = None;
        self.shared_state.clear();
        self.shape_logs.clear();
        for algo in &mut self.algorithms {
            algo.on_reset();
        }
        self.phase_info_dirty = true;
        self.profiler.reset();
    }

    /// 从当前位置执行到底
    pub fn run_all(
        &mut self,
        world: &mut World,
        profile: &WorldProfile,
        blocks: &[BlockDefinition],
    ) -> Result<(), String> {
        self.profiler.start_generation();
        while !self.is_complete() {
            self.step_forward_sub(world, profile, blocks)?;
        }
        Ok(())
    }

    // ── 性能分析器访问 ─────────────────────────────

    /// 获取性能分析器的只读引用
    pub fn profiler(&self) -> &PerfProfiler {
        &self.profiler
    }

    /// 获取性能分析报告
    pub fn performance_report(&self) -> String {
        self.profiler.report()
    }

    // ── 快照支持 ────────────────────────────────────────────

    /// 收集当前运行状态为快照
    pub fn collect_snapshot(
        &self,
        world_size: &str,
        layers: &[crate::core::layer::LayerDefinition],
    ) -> super::snapshot::WorldSnapshot {
        super::snapshot::WorldSnapshot::collect(
            self.seed,
            world_size,
            layers,
            &self.algorithms,
        )
    }

    /// 从快照恢复算法参数（seed 和 world_size 由调用方处理）
    pub fn restore_from_snapshot(&mut self, snapshot: &super::snapshot::WorldSnapshot) {
        for algo_state in &snapshot.algorithms {
            // 按 algorithm_id 匹配并恢复参数
            for algo in &mut self.algorithms {
                if algo.meta().id == algo_state.algorithm_id {
                    algo.set_params(&algo_state.params);
                }
            }
        }
        self.phase_info_dirty = true;
    }

    // ── UI 信息 ─────────────────────────────────────────────

    /// 构建控制面板需要的阶段/步骤快照列表（带缓存，仅步骤变化时重建）
    pub fn phase_info_list(&mut self) -> &[PhaseInfo] {
        let executed = self.executed_sub_steps();
        if !self.phase_info_dirty && self.cached_phase_info_executed == executed {
            return &self.cached_phase_info;
        }

        let mut flat = 0usize;
        self.cached_phase_info = self
            .algorithms
            .iter()
            .enumerate()
            .map(|(pi, algo)| {
                let meta = algo.meta();
                let phase_start = flat;

                let sub_infos: Vec<SubStepInfo> = meta
                    .steps
                    .iter()
                    .enumerate()
                    .map(|(si, step_meta)| {
                        let status = if flat < executed {
                            StepStatus::Completed
                        } else if flat == executed {
                            StepStatus::Current
                        } else {
                            StepStatus::Pending
                        };
                        flat += 1;
                        SubStepInfo {
                            display_id: format!("{}.{}", pi + 1, si),
                            name: step_meta.name.clone(),
                            description: step_meta.description.clone(),
                            doc_url: step_meta.doc_url.clone(),
                            status,
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
                    display_index: (pi + 1) as u32,
                    algorithm_id: meta.id.clone(),
                    name: meta.name.clone(),
                    description: meta.description.clone(),
                    has_params: !meta.params.is_empty(),
                    sub_steps: sub_infos,
                    status: phase_status,
                }
            })
            .collect();

        self.cached_phase_info_executed = executed;
        self.phase_info_dirty = false;
        &self.cached_phase_info
    }

    // ── 内部方法 ────────────────────────────────────────────

    fn flat_to_position(&self, flat: usize) -> (usize, usize) {
        let mut remaining = flat;
        for (pi, &count) in self.step_counts.iter().enumerate() {
            if remaining < count {
                return (pi, remaining);
            }
            remaining -= count;
        }
        (self.algorithms.len(), 0)
    }

    fn replay_to_flat(
        &mut self,
        target_flat: usize,
        world: &mut World,
        profile: &WorldProfile,
        blocks: &[BlockDefinition],
    ) -> Result<(), String> {
        *world = World::new_air(world.width, world.height);
        self.biome_map = None;
        self.shared_state.clear();
        self.current_phase = 0;
        self.current_sub = 0;
        for algo in &mut self.algorithms {
            algo.on_reset();
        }

        for _ in 0..target_flat {
            self.step_forward_sub(world, profile, blocks)?;
        }

        Ok(())
    }
}

/// 从主种子派生每步的确定性子种子
///
/// 混入世界尺寸，使同一种子+不同世界尺寸得到不同的生成结果（与泰拉瑞亚行为一致）。
fn derive_step_seed(master: u64, step_index: usize, world_width: u32, world_height: u32) -> u64 {
    let size_mix = (world_width as u64) << 32 | (world_height as u64);
    master
        .wrapping_add(step_index as u64)
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407)
        .wrapping_add(size_mix.wrapping_mul(2_862_933_555_777_941_757))
}
