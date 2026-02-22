//! # 引擎自动优化模块
//!
//! 提供运行时性能分析和自动调优功能：
//!
//! - **AdaptiveBatchSize**: 根据实际步骤执行时间动态调整每帧批量大小
//! - **PerfProfiler**: 记录每步执行时间，识别瓶颈并输出分析报告
//! - **ParallelReplay**: 并行回放优化（利用无副作用步骤的独立性）
//!
//! ## 设计原则
//!
//! - 引擎无需手动配置优化参数——所有调优基于运行时测量
//! - 对现有 Pipeline 接口零侵入——通过组合而非继承

use std::collections::HashMap;
use std::time::{Duration, Instant};

// ═══════════════════════════════════════════════════════════
// 自适应批量大小控制
// ═══════════════════════════════════════════════════════════

/// 自适应批量大小控制器
///
/// 在增量执行（run_to_end）期间，动态调整每帧执行的步骤数，
/// 将帧时间维持在目标范围内（默认 8-16ms，即 60-120fps）。
pub struct AdaptiveBatchSize {
    /// 当前每帧批量大小
    current_batch: usize,
    /// 目标帧时间下限（ms）
    target_min_ms: f64,
    /// 目标帧时间上限（ms）
    target_max_ms: f64,
    /// 最小批量
    min_batch: usize,
    /// 最大批量
    max_batch: usize,
    /// 上一帧的执行时间
    last_frame_duration: Duration,
    /// 指数移动平均的帧时间（用于平滑抖动）
    ema_frame_ms: f64,
    /// EMA 平滑系数
    alpha: f64,
}

impl Default for AdaptiveBatchSize {
    fn default() -> Self {
        Self {
            current_batch: 3,
            target_min_ms: 8.0,
            target_max_ms: 16.0,
            min_batch: 1,
            max_batch: 64,
            last_frame_duration: Duration::ZERO,
            ema_frame_ms: 10.0,
            alpha: 0.3,
        }
    }
}

impl AdaptiveBatchSize {
    pub fn new() -> Self {
        Self::default()
    }

    /// 配置目标帧率范围
    pub fn with_target_fps(mut self, min_fps: f64, max_fps: f64) -> Self {
        self.target_max_ms = 1000.0 / min_fps;
        self.target_min_ms = 1000.0 / max_fps;
        self
    }

    /// 获取当前推荐的批量大小
    pub fn batch_size(&self) -> usize {
        self.current_batch
    }

    /// 报告一帧的执行时间，自动调整批量大小
    pub fn report_frame(&mut self, duration: Duration) {
        self.last_frame_duration = duration;
        let frame_ms = duration.as_secs_f64() * 1000.0;

        // 更新 EMA
        self.ema_frame_ms = self.alpha * frame_ms + (1.0 - self.alpha) * self.ema_frame_ms;

        // 调整策略：
        // - 帧时间 < target_min → 可以做更多（加倍或 +50%）
        // - 帧时间 > target_max → 需要减少（减半或 -25%）
        // - 在目标范围内 → 保持不变
        if self.ema_frame_ms < self.target_min_ms {
            // 帧很快，可以增加批量
            let ratio = (self.target_min_ms / self.ema_frame_ms).min(2.0);
            self.current_batch = ((self.current_batch as f64 * ratio).ceil() as usize)
                .max(self.current_batch + 1)
                .min(self.max_batch);
        } else if self.ema_frame_ms > self.target_max_ms {
            // 帧太慢，需要减少批量
            let ratio = (self.target_max_ms / self.ema_frame_ms).max(0.5);
            self.current_batch = ((self.current_batch as f64 * ratio).floor() as usize)
                .max(self.min_batch);
        }
        // 在目标范围内：不调整
    }

    /// 获取上一帧执行时间
    pub fn last_frame_duration(&self) -> Duration {
        self.last_frame_duration
    }

    /// 获取 EMA 平滑帧时间（ms）
    pub fn ema_frame_ms(&self) -> f64 {
        self.ema_frame_ms
    }

    /// 重置状态
    pub fn reset(&mut self) {
        self.current_batch = 3;
        self.ema_frame_ms = 10.0;
        self.last_frame_duration = Duration::ZERO;
    }
}

// ═══════════════════════════════════════════════════════════
// 性能分析器
// ═══════════════════════════════════════════════════════════

/// 单个步骤的性能采样
#[derive(Debug, Clone)]
pub struct StepProfile {
    /// 步骤名称
    pub name: String,
    /// 执行次数
    pub run_count: u32,
    /// 总执行时间
    pub total_duration: Duration,
    /// 最小执行时间
    pub min_duration: Duration,
    /// 最大执行时间
    pub max_duration: Duration,
}

impl StepProfile {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            run_count: 0,
            total_duration: Duration::ZERO,
            min_duration: Duration::MAX,
            max_duration: Duration::ZERO,
        }
    }

    fn record(&mut self, duration: Duration) {
        self.run_count += 1;
        self.total_duration += duration;
        self.min_duration = self.min_duration.min(duration);
        self.max_duration = self.max_duration.max(duration);
    }

    /// 平均执行时间
    pub fn avg_duration(&self) -> Duration {
        if self.run_count == 0 {
            Duration::ZERO
        } else {
            self.total_duration / self.run_count
        }
    }
}

/// 性能分析器
///
/// 跟踪每个子步骤的执行时间，提供性能分析报告。
pub struct PerfProfiler {
    /// 步骤性能数据（key = flat_index）
    steps: HashMap<usize, StepProfile>,
    /// 最近 N 帧的帧时间（用于 FPS 统计）
    recent_frame_times: Vec<Duration>,
    /// 最大记录帧数
    max_recent_frames: usize,
    /// 总执行时间
    total_duration: Duration,
    /// 启动时间
    start_time: Option<Instant>,
}

impl Default for PerfProfiler {
    fn default() -> Self {
        Self {
            steps: HashMap::new(),
            recent_frame_times: Vec::new(),
            max_recent_frames: 120,
            total_duration: Duration::ZERO,
            start_time: None,
        }
    }
}

impl PerfProfiler {
    pub fn new() -> Self {
        Self::default()
    }

    /// 开始一次生成计时
    pub fn start_generation(&mut self) {
        self.start_time = Some(Instant::now());
    }

    /// 记录一个步骤的执行时间
    pub fn record_step(&mut self, flat_index: usize, name: &str, duration: Duration) {
        self.total_duration += duration;
        self.steps
            .entry(flat_index)
            .or_insert_with(|| StepProfile::new(name))
            .record(duration);
    }

    /// 记录一帧时间
    pub fn record_frame(&mut self, duration: Duration) {
        self.recent_frame_times.push(duration);
        if self.recent_frame_times.len() > self.max_recent_frames {
            self.recent_frame_times.remove(0);
        }
    }

    /// 获取最近帧的平均 FPS
    pub fn recent_avg_fps(&self) -> f64 {
        if self.recent_frame_times.is_empty() {
            return 0.0;
        }
        let total: Duration = self.recent_frame_times.iter().sum();
        let avg_secs = total.as_secs_f64() / self.recent_frame_times.len() as f64;
        if avg_secs > 0.0 { 1.0 / avg_secs } else { 0.0 }
    }

    /// 获取总生成耗时
    pub fn total_generation_time(&self) -> Duration {
        if let Some(start) = self.start_time {
            start.elapsed()
        } else {
            self.total_duration
        }
    }

    /// 获取最慢的 N 个步骤
    pub fn slowest_steps(&self, n: usize) -> Vec<&StepProfile> {
        let mut profiles: Vec<&StepProfile> = self.steps.values().collect();
        profiles.sort_by(|a, b| b.avg_duration().cmp(&a.avg_duration()));
        profiles.truncate(n);
        profiles
    }

    /// 获取所有步骤的性能数据（按 flat_index 排序）
    pub fn all_steps_sorted(&self) -> Vec<(usize, &StepProfile)> {
        let mut entries: Vec<(usize, &StepProfile)> = self.steps.iter().map(|(&k, v)| (k, v)).collect();
        entries.sort_by_key(|(k, _)| *k);
        entries
    }

    /// 生成性能报告字符串
    pub fn report(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("=== 性能分析报告 ==="));
        lines.push(format!("总生成耗时: {:.1}ms", self.total_generation_time().as_secs_f64() * 1000.0));

        if !self.steps.is_empty() {
            lines.push(String::new());
            lines.push(format!("{:<6} {:<30} {:>10} {:>10} {:>10}", "步骤", "名称", "平均(ms)", "最小(ms)", "最大(ms)"));
            lines.push("-".repeat(70));

            for (idx, profile) in self.all_steps_sorted() {
                lines.push(format!(
                    "{:<6} {:<30} {:>10.2} {:>10.2} {:>10.2}",
                    idx,
                    profile.name,
                    profile.avg_duration().as_secs_f64() * 1000.0,
                    profile.min_duration.as_secs_f64() * 1000.0,
                    profile.max_duration.as_secs_f64() * 1000.0,
                ));
            }

            lines.push(String::new());
            lines.push(format!("最慢步骤 TOP 3:"));
            for (i, profile) in self.slowest_steps(3).iter().enumerate() {
                lines.push(format!(
                    "  {}. {} — 平均 {:.2}ms",
                    i + 1,
                    profile.name,
                    profile.avg_duration().as_secs_f64() * 1000.0,
                ));
            }
        }

        lines.join("\n")
    }

    /// 重置所有数据
    pub fn reset(&mut self) {
        self.steps.clear();
        self.recent_frame_times.clear();
        self.total_duration = Duration::ZERO;
        self.start_time = None;
    }
}

// ═══════════════════════════════════════════════════════════
// 纹理更新频率控制
// ═══════════════════════════════════════════════════════════

/// 智能纹理更新控制器
///
/// 在增量执行期间，根据世界大小和帧率动态调整纹理刷新频率，
/// 避免过于频繁的 CPU→GPU 上传成为瓶颈。
pub struct TextureUpdateThrottle {
    /// 帧计数器
    frame_counter: usize,
    /// 当前刷新间隔（每 N 帧刷新一次）  
    refresh_interval: usize,
    /// 世界总像素数（用于估算上传开销）
    world_pixels: usize,
}

impl TextureUpdateThrottle {
    pub fn new(world_width: u32, world_height: u32) -> Self {
        let world_pixels = (world_width as usize) * (world_height as usize);
        // 根据世界大小自动调整：
        // < 100万像素 → 每 3 帧      (small world)
        // 100-400万像素 → 每 5 帧     (medium world)
        // > 400万像素 → 每 8 帧       (large world)
        let refresh_interval = if world_pixels < 1_000_000 {
            3
        } else if world_pixels < 4_000_000 {
            5
        } else {
            8
        };

        Self {
            frame_counter: 0,
            refresh_interval,
            world_pixels,
        }
    }

    /// 每帧调用，返回是否应该刷新纹理
    pub fn tick(&mut self, is_final: bool) -> bool {
        self.frame_counter += 1;
        is_final || self.frame_counter % self.refresh_interval == 0
    }

    /// 重置计数器
    pub fn reset(&mut self) {
        self.frame_counter = 0;
    }

    /// 当前刷新间隔
    pub fn refresh_interval(&self) -> usize {
        self.refresh_interval
    }

    /// 世界像素数
    pub fn world_pixels(&self) -> usize {
        self.world_pixels
    }

    /// 调整刷新间隔（用于根据实际帧率微调）
    pub fn adjust_interval(&mut self, avg_frame_ms: f64) {
        if avg_frame_ms > 20.0 {
            // 帧率低于 50fps，减少纹理更新频率
            self.refresh_interval = (self.refresh_interval + 1).min(16);
        } else if avg_frame_ms < 8.0 && self.refresh_interval > 2 {
            // 帧率充裕，可以更频繁地更新
            self.refresh_interval -= 1;
        }
    }
}
