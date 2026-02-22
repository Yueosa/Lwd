//! # 性能面板
//!
//! 以 egui 窗口展示：
//! - 当前 EngineConfig 参数（可编辑）
//! - 最近一次生成的性能报告
//! - 历史性能日志列表

use egui::Window;

use crate::generation::optimizer::PerfProfiler;
use crate::storage::engine_config::EngineConfig;
use crate::storage::perf_log;
use crate::ui::theme;

/// 显示性能面板窗口。
///
/// 返回 `true` 表示 EngineConfig 被修改（调用方应保存并应用）。
pub fn show_perf_panel_window(
    ctx: &egui::Context,
    is_open: &mut bool,
    config: &mut EngineConfig,
    profiler: &PerfProfiler,
) -> bool {
    let mut changed = false;

    Window::new("⚙ 性能面板")
        .open(is_open)
        .default_width(480.0)
        .default_height(560.0)
        .vscroll(true)
        .show(ctx, |ui| {
            // ── 引擎参数 ──
            ui.colored_label(theme::PINK, "◈ 引擎参数");
            ui.add_space(4.0);

            egui::Grid::new("engine_params")
                .num_columns(2)
                .spacing([12.0, 4.0])
                .show(ui, |ui| {
                    // 并行阈值
                    ui.label("并行像素阈值");
                    let mut val = config.parallel_pixel_threshold as f32;
                    if ui.add(egui::Slider::new(&mut val, 5000.0..=500_000.0)
                        .logarithmic(true)
                        .suffix(" px")
                    ).changed() {
                        config.parallel_pixel_threshold = val as i64;
                        changed = true;
                    }
                    ui.end_row();

                    // Batch 参数
                    ui.label("初始 Batch");
                    let mut v = config.batch_initial as u32;
                    if ui.add(egui::DragValue::new(&mut v).clamp_range(1..=64)).changed() {
                        config.batch_initial = v as usize;
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("目标帧时间 (ms)");
                    ui.horizontal(|ui| {
                        let mut lo = config.batch_target_min_ms as f32;
                        let mut hi = config.batch_target_max_ms as f32;
                        ui.label("下限");
                        if ui.add(egui::DragValue::new(&mut lo).clamp_range(1.0..=50.0).speed(0.5)).changed() {
                            config.batch_target_min_ms = lo as f64;
                            changed = true;
                        }
                        ui.label("上限");
                        if ui.add(egui::DragValue::new(&mut hi).clamp_range(1.0..=100.0).speed(0.5)).changed() {
                            config.batch_target_max_ms = hi as f64;
                            changed = true;
                        }
                    });
                    ui.end_row();

                    ui.label("Batch 范围");
                    ui.horizontal(|ui| {
                        let mut lo = config.batch_min as u32;
                        let mut hi = config.batch_max as u32;
                        ui.label("最小");
                        if ui.add(egui::DragValue::new(&mut lo).clamp_range(1..=32)).changed() {
                            config.batch_min = lo as usize;
                            changed = true;
                        }
                        ui.label("最大");
                        if ui.add(egui::DragValue::new(&mut hi).clamp_range(1..=256)).changed() {
                            config.batch_max = hi as usize;
                            changed = true;
                        }
                    });
                    ui.end_row();

                    ui.label("EMA 平滑系数");
                    let mut alpha = config.batch_ema_alpha as f32;
                    if ui.add(egui::Slider::new(&mut alpha, 0.05..=0.9)).changed() {
                        config.batch_ema_alpha = alpha as f64;
                        changed = true;
                    }
                    ui.end_row();

                    // 纹理节流
                    ui.colored_label(theme::BLUE_LIGHT, "纹理节流");
                    ui.label("");
                    ui.end_row();

                    ui.label("小世界阈值 (px)");
                    let mut v = config.throttle_small_threshold as u32;
                    if ui.add(egui::DragValue::new(&mut v).clamp_range(100_000..=10_000_000).speed(50000)).changed() {
                        config.throttle_small_threshold = v as usize;
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("大世界阈值 (px)");
                    let mut v = config.throttle_large_threshold as u32;
                    if ui.add(egui::DragValue::new(&mut v).clamp_range(100_000..=50_000_000).speed(100000)).changed() {
                        config.throttle_large_threshold = v as usize;
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("刷新间隔 (小/中/大)");
                    ui.horizontal(|ui| {
                        let mut s = config.throttle_refresh_small as u32;
                        let mut m = config.throttle_refresh_medium as u32;
                        let mut l = config.throttle_refresh_large as u32;
                        let c1 = ui.add(egui::DragValue::new(&mut s).clamp_range(1..=32)).changed();
                        let c2 = ui.add(egui::DragValue::new(&mut m).clamp_range(1..=32)).changed();
                        let c3 = ui.add(egui::DragValue::new(&mut l).clamp_range(1..=32)).changed();
                        if c1 || c2 || c3 {
                            config.throttle_refresh_small = s as usize;
                            config.throttle_refresh_medium = m as usize;
                            config.throttle_refresh_large = l as usize;
                            changed = true;
                        }
                    });
                    ui.end_row();

                    // 日志保留
                    ui.label("日志最大保留数");
                    let mut v = config.perf_log_max_files as u32;
                    if ui.add(egui::DragValue::new(&mut v).clamp_range(1..=1000)).changed() {
                        config.perf_log_max_files = v as usize;
                        changed = true;
                    }
                    ui.end_row();
                });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("重新校准").on_hover_text("运行微基准测试，自动确定最优参数").clicked() {
                    config.calibrate();
                    config.save();
                    changed = true;
                }
                if ui.button("恢复默认").clicked() {
                    *config = EngineConfig::default();
                    config.calibrated = true; // 标记已处理，避免下次重校
                    changed = true;
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);

            // ── 当前性能报告 ──
            ui.colored_label(theme::PINK, "◈ 当前生成性能");
            ui.add_space(4.0);

            let total_ms = profiler.total_generation_time().as_secs_f64() * 1000.0;
            if total_ms > 0.0 {
                ui.label(format!("总耗时: {:.1}ms", total_ms));
                ui.add_space(2.0);

                let steps = profiler.all_steps_sorted();
                if !steps.is_empty() {
                    egui::Grid::new("perf_steps")
                        .num_columns(4)
                        .spacing([8.0, 2.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.colored_label(theme::BLUE_LIGHT, "步骤");
                            ui.colored_label(theme::BLUE_LIGHT, "名称");
                            ui.colored_label(theme::BLUE_LIGHT, "平均(ms)");
                            ui.colored_label(theme::BLUE_LIGHT, "最大(ms)");
                            ui.end_row();

                            for (idx, sp) in &steps {
                                ui.label(format!("{idx}"));
                                ui.label(&sp.name);
                                ui.label(format!("{:.2}", sp.avg_duration().as_secs_f64() * 1000.0));
                                ui.label(format!("{:.2}", sp.max_duration.as_secs_f64() * 1000.0));
                                ui.end_row();
                            }
                        });
                }
            } else {
                ui.label("尚未生成");
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);

            // ── 历史日志 ──
            ui.colored_label(theme::PINK, "◈ 历史日志");
            ui.add_space(4.0);

            let logs = perf_log::list_entries();
            if logs.is_empty() {
                ui.label("暂无日志");
            } else {
                egui::Grid::new("perf_logs")
                    .num_columns(3)
                    .spacing([8.0, 2.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.colored_label(theme::BLUE_LIGHT, "时间");
                        ui.colored_label(theme::BLUE_LIGHT, "尺寸");
                        ui.colored_label(theme::BLUE_LIGHT, "耗时(ms)");
                        ui.end_row();

                        for log in logs.iter().take(20) {
                            ui.label(&log.timestamp);
                            ui.label(&log.world_size);
                            ui.label(format!("{:.1}", log.total_ms));
                            ui.end_row();
                        }
                    });
            }
        });

    changed
}
