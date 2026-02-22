//! # 引擎配置（EngineConfig）
//!
//! 集中管理所有引擎调优参数，消除硬编码。
//! 持久化到 runtime.json 的 `"engine"` 字段。
//!
//! ## 自校准
//!
//! 首次运行（或 runtime.json 中没有 `"engine"` 字段）时，
//! 调用 `EngineConfig::calibrate()` 执行微基准测试，
//! 测量 rayon 线程池启动开销，自动设置 `parallel_pixel_threshold`。

use serde::{Deserialize, Serialize};

use super::runtime;

// ═══════════════════════════════════════════════════════════
// 配置结构
// ═══════════════════════════════════════════════════════════

/// 引擎调优参数——所有可调数值的唯一来源。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EngineConfig {
    // ── 并行化 ──
    /// 像素数超过此阈值时启用 rayon 并行（geometry fill / world fill_rect）
    pub parallel_pixel_threshold: i64,

    // ── 自适应批量 ──
    /// 增量执行初始 batch 大小
    pub batch_initial: usize,
    /// 目标帧时间下限（ms），低于此值增大 batch
    pub batch_target_min_ms: f64,
    /// 目标帧时间上限（ms），高于此值减小 batch
    pub batch_target_max_ms: f64,
    /// 最小 batch
    pub batch_min: usize,
    /// 最大 batch
    pub batch_max: usize,
    /// EMA 平滑系数 (0, 1)
    pub batch_ema_alpha: f64,

    // ── 纹理节流 ──
    /// 小世界像素阈值（低于此值使用 refresh_small）
    pub throttle_small_threshold: usize,
    /// 大世界像素阈值（高于此值使用 refresh_large）
    pub throttle_large_threshold: usize,
    /// 小世界纹理刷新间隔（帧）
    pub throttle_refresh_small: usize,
    /// 中世界纹理刷新间隔（帧）
    pub throttle_refresh_medium: usize,
    /// 大世界纹理刷新间隔（帧）
    pub throttle_refresh_large: usize,

    // ── 性能日志 ──
    /// 日志文件最大保留数量
    pub perf_log_max_files: usize,

    // ── 元数据 ──
    /// 是否已经过自校准
    pub calibrated: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            parallel_pixel_threshold: 50_000,

            batch_initial: 3,
            batch_target_min_ms: 8.0,
            batch_target_max_ms: 16.0,
            batch_min: 1,
            batch_max: 64,
            batch_ema_alpha: 0.3,

            throttle_small_threshold: 1_000_000,
            throttle_large_threshold: 4_000_000,
            throttle_refresh_small: 3,
            throttle_refresh_medium: 5,
            throttle_refresh_large: 8,

            perf_log_max_files: 100,

            calibrated: false,
        }
    }
}

impl EngineConfig {
    /// 从 runtime.json 加载，如不存在则返回默认值。
    pub fn load() -> Self {
        runtime::load_field("engine")
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default()
    }

    /// 保存到 runtime.json 的 `"engine"` 字段。
    pub fn save(&self) {
        if let Ok(v) = serde_json::to_value(self) {
            let _ = runtime::merge_field("engine", v);
        }
    }

    /// 首次运行自校准：测量 rayon 并行开销，确定 `parallel_pixel_threshold`。
    ///
    /// 做法：用一个简单的逐像素任务，分别串行和并行跑若干次，
    /// 找到并行开始比串行快的交叉点。
    pub fn calibrate(&mut self) {
        use rayon::prelude::*;
        use std::time::Instant;

        // 用几个不同的数据量来测量
        let test_sizes: &[usize] = &[10_000, 25_000, 50_000, 100_000, 200_000];
        let warmup_iters = 2;
        let bench_iters = 5;

        let mut threshold = 50_000i64; // fallback

        for &size in test_sizes {
            let mut data = vec![0u8; size];

            // warmup
            for _ in 0..warmup_iters {
                data.iter_mut().for_each(|v| *v = v.wrapping_add(1));
                data.par_chunks_mut(1024).for_each(|chunk| {
                    chunk.iter_mut().for_each(|v| *v = v.wrapping_add(1));
                });
            }

            // bench serial
            let t0 = Instant::now();
            for _ in 0..bench_iters {
                data.iter_mut().for_each(|v| *v = v.wrapping_add(1));
            }
            let serial_ns = t0.elapsed().as_nanos() / bench_iters as u128;

            // bench parallel
            let t0 = Instant::now();
            for _ in 0..bench_iters {
                data.par_chunks_mut(1024).for_each(|chunk| {
                    chunk.iter_mut().for_each(|v| *v = v.wrapping_add(1));
                });
            }
            let parallel_ns = t0.elapsed().as_nanos() / bench_iters as u128;

            // 找到并行开始获益的最小 size
            if parallel_ns < serial_ns {
                threshold = size as i64;
                break;
            }
        }

        // 加一点安全余量（线程切换 + shape.contains 比纯写入更重）
        self.parallel_pixel_threshold = (threshold as f64 * 0.8) as i64;
        self.calibrated = true;

        eprintln!(
            "[engine] 自校准完成: parallel_pixel_threshold = {}",
            self.parallel_pixel_threshold
        );
    }

    /// 如果尚未校准，执行校准并保存。
    pub fn ensure_calibrated(&mut self) {
        if !self.calibrated {
            self.calibrate();
            self.save();
        }
    }
}
