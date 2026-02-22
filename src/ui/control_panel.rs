use egui::{Align, Layout, Rect, ScrollArea, Ui, Vec2};

use crate::config::world::WorldConfig;
use crate::generation::{PhaseInfo, StepStatus};
use crate::ui::theme;

// ── action returned to the app ──────────────────────────────

#[derive(Debug, Clone)]
pub struct ControlAction {
    pub zoom_in: bool,
    pub zoom_out: bool,
    pub zoom_reset: bool,
    /// 小步前进 (+0.1)
    pub step_forward_sub: bool,
    /// 大步前进 (+1.0, 执行完当前 phase)
    pub step_forward_phase: bool,
    /// 小步后退 (-0.1)
    pub step_backward_sub: bool,
    /// 大步后退 (-1.0, 回退到当前 phase 开头)
    pub step_backward_phase: bool,
    pub run_all: bool,
    pub reset_and_step: bool,
    pub open_overlay_config: bool,
    pub open_layer_config: bool,
    /// 打开当前步骤的算法配置面板
    pub open_step_config: bool,
    /// 打开几何预览窗口
    pub open_geo_preview: bool,
    /// 打开图形 API 沙箱窗口
    pub open_shape_sandbox: bool,
    /// 导出 PNG
    pub export_png: bool,
    /// 导出 .lwd 存档
    pub export_lwd: bool,
    /// 导入 .lwd 存档
    pub import_lwd: bool,
    /// 应用手动输入的种子
    pub apply_seed: bool,
    /// 打开性能面板
    pub open_perf_panel: bool,
}

impl ControlAction {
    pub fn none() -> Self {
        Self {
            zoom_in: false,
            zoom_out: false,
            zoom_reset: false,
            step_forward_sub: false,
            step_forward_phase: false,
            step_backward_sub: false,
            step_backward_phase: false,
            run_all: false,
            reset_and_step: false,
            open_overlay_config: false,
            open_layer_config: false,
            open_step_config: false,
            open_geo_preview: false,
            open_shape_sandbox: false,
            export_png: false,
            export_lwd: false,
            import_lwd: false,
            apply_seed: false,
            open_perf_panel: false,
        }
    }
}

// ── world size enum ─────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldSizeSelection {
    Small,
    Medium,
    Large,
    Custom,
}

impl Default for WorldSizeSelection {
    fn default() -> Self {
        Self::Small
    }
}

// ── panel rendering ─────────────────────────────────────────

pub fn show_control_panel(
    ui: &mut Ui,
    world_size: &mut WorldSizeSelection,
    custom_width: &mut String,
    custom_height: &mut String,
    world_cfg: &WorldConfig,
    seed_input: &mut String,
    phase_info: &[PhaseInfo],
    executed: usize,
    total: usize,
) -> ControlAction {
    let mut action = ControlAction::none();

    ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui: &mut Ui| {

    // ── 标题 ──
    ui.add_space(6.0);
    ui.with_layout(Layout::top_down(Align::Center), |ui| {
        ui.colored_label(theme::PINK, egui::RichText::new("✿ Lian World ✿").heading());
    });
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);

    // ── 世界尺寸 ──
    ui.colored_label(theme::BLUE_LIGHT, "◈ 世界尺寸");
    // 动态生成预设尺寸 radio buttons
    let presets: &[(&str, WorldSizeSelection)] = &[
        ("small", WorldSizeSelection::Small),
        ("medium", WorldSizeSelection::Medium),
        ("large", WorldSizeSelection::Large),
    ];
    for &(key, variant) in presets {
        if let Some(size_cfg) = world_cfg.world_sizes.get(key) {
            if let (Some(w), Some(h)) = (size_cfg.width, size_cfg.height) {
                let label = format!("{} ({}×{})", size_cfg.description, w, h);
                ui.radio_value(world_size, variant, label);
            }
        }
    }
    // 自定义尺寸
    ui.radio_value(world_size, WorldSizeSelection::Custom, "自定义");
    if *world_size == WorldSizeSelection::Custom {
        ui.horizontal(|ui| {
            ui.label("宽:");
            ui.add(egui::TextEdit::singleline(custom_width)
                .hint_text("4200")
                .desired_width(60.0));
            ui.label("× 高:");
            ui.add(egui::TextEdit::singleline(custom_height)
                .hint_text("1200")
                .desired_width(60.0));
        });
    }

    ui.add_space(2.0);
    ui.separator();
    ui.add_space(4.0);

    // ── 种子输入 ──
    ui.colored_label(theme::BLUE_LIGHT, "◈ 种子");
    ui.horizontal(|ui| {
        let text_edit = egui::TextEdit::singleline(seed_input)
            .hint_text("输入种子 (十六进制/十进制)")
            .desired_width(140.0);
        let resp = ui.add(text_edit);
        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            action.apply_seed = true;
        }
        if ui.button("OK").on_hover_text("应用种子并重置到第0步").clicked() {
            action.apply_seed = true;
        }
    });

    ui.add_space(2.0);
    ui.separator();
    ui.add_space(4.0);

    // ── 生成进度（自定义粉蓝渐变进度条）──
    ui.colored_label(theme::BLUE_LIGHT, "◈ 生成进度");
    let progress = if total == 0 {
        0.0
    } else {
        executed as f32 / total as f32
    };
    
    // 自定义渐变进度条
    let desired_size = Vec2::new(ui.available_width(), 18.0);
    let (rect, _response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
    let painter = ui.painter();
    
    // 背景
    painter.rect_filled(rect, 4.0, theme::BG_WIDGET);
    
    // 填充条 — 粉→蓝渐变
    if progress > 0.0 {
        let fill_width = rect.width() * progress;
        let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_width, rect.height()));
        
        // 用当前进度对应的颜色填充
        let fill_color = theme::progress_color(progress);
        painter.rect_filled(fill_rect, 4.0, fill_color);
    }
    
    // 进度文字
    let text = format!("{:.0}%", progress * 100.0);
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        &text,
        egui::FontId::proportional(11.0),
        theme::WHITE,
    );
    
    ui.label(egui::RichText::new(format!("子步骤: {executed}/{total}")).color(theme::TEXT_SECONDARY).small());

    ui.add_space(2.0);
    ui.separator();
    ui.add_space(4.0);

    // ── 步进控制 ──
    ui.colored_label(theme::BLUE_LIGHT, "◈ 步进控制");
    ui.horizontal(|ui| {
        if ui
            .add_enabled(executed > 0, egui::Button::new(
                egui::RichText::new("⏮").color(theme::PINK)
            ))
            .on_hover_text("大步后退 (回到阶段开头)")
            .clicked()
        {
            action.step_backward_phase = true;
        }
        if ui
            .add_enabled(executed > 0, egui::Button::new(
                egui::RichText::new("◂").color(theme::PINK_LIGHT)
            ))
            .on_hover_text("小步后退")
            .clicked()
        {
            action.step_backward_sub = true;
        }
        if ui
            .add_enabled(executed < total, egui::Button::new(
                egui::RichText::new("▸").color(theme::BLUE_LIGHT)
            ))
            .on_hover_text("小步前进")
            .clicked()
        {
            action.step_forward_sub = true;
        }
        if ui
            .add_enabled(executed < total, egui::Button::new(
                egui::RichText::new("⏭").color(theme::BLUE)
            ))
            .on_hover_text("大步前进 (执行完当前阶段)")
            .clicked()
        {
            action.step_forward_phase = true;
        }
    });

    ui.add_space(2.0);
    ui.separator();
    ui.add_space(4.0);

    // ── 步骤列表 ──
    ui.colored_label(theme::BLUE_LIGHT, "◈ 步骤列表");
    let step_list_max_h = (ui.available_height() * 0.35).clamp(100.0, 300.0);
    ScrollArea::vertical()
        .id_source("step_list_scroll")
        .max_height(step_list_max_h)
        .show(ui, |ui| {
            for phase in phase_info {
                let (phase_prefix, phase_color) = match phase.status {
                    StepStatus::Completed => (theme::STEP_COMPLETED_SYMBOL, theme::STEP_COMPLETED_COLOR),
                    StepStatus::Current => (theme::STEP_CURRENT_SYMBOL, theme::STEP_CURRENT_COLOR),
                    StepStatus::Pending => (theme::STEP_PENDING_SYMBOL, theme::STEP_PENDING_COLOR),
                };
                let phase_label = format!(
                    "{phase_prefix} {}. {}",
                    phase.display_index, phase.name
                );
                let resp = ui.colored_label(phase_color, &phase_label);
                if resp.hovered() {
                    resp.on_hover_text(&phase.description);
                }

                for sub in &phase.sub_steps {
                    let (sub_prefix, sub_color) = match sub.status {
                        StepStatus::Completed => (theme::SUB_COMPLETED_SYMBOL, theme::SUB_COMPLETED_COLOR),
                        StepStatus::Current => (theme::SUB_CURRENT_SYMBOL, theme::SUB_CURRENT_COLOR),
                        StepStatus::Pending => (theme::SUB_PENDING_SYMBOL, theme::SUB_PENDING_COLOR),
                    };
                    
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        let sub_label = format!("{sub_prefix} {} {}", sub.display_id, sub.name);
                        let resp = ui.colored_label(sub_color, &sub_label);
                        
                        if resp.hovered() {
                            resp.on_hover_ui(|ui| {
                                ui.label(&sub.description);
                                if let Some(url) = &sub.doc_url {
                                    ui.hyperlink_to("[Doc] 查看算法文档", url);
                                }
                            });
                        }
                    });
                }
            }
        });

    ui.add_space(4.0);
    ui.separator();
    ui.add_space(6.0);

    // ── 生成操作 ──
    ui.colored_label(theme::BLUE_LIGHT, "◈ 生成操作");
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        if ui.button(egui::RichText::new("✦ 一键生成").color(theme::PINK_LIGHT))
            .on_hover_text("新种子 → 重置 → 执行全部步骤").clicked() {
            action.reset_and_step = true;
            action.run_all = true;
        }
        if ui.button(egui::RichText::new("↻ 重新初始化").color(theme::BLUE_LIGHT))
            .on_hover_text("新种子 → 重置到第0步").clicked() {
            action.reset_and_step = true;
        }
    });
    ui.add_space(2.0);
    if ui
        .add_enabled(executed < total, egui::Button::new(
            egui::RichText::new("▶▶ 执行到底").color(theme::WHITE)
        ))
        .on_hover_text("从当前步骤一直执行到最后")
        .clicked()
    {
        action.run_all = true;
    }
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        if ui.button(egui::RichText::new("≡ 算法参数").color(theme::TEXT_SECONDARY))
            .on_hover_text("打开当前步骤的算法参数配置面板").clicked() {
            action.open_step_config = true;
        }
        if ui.button(egui::RichText::new("📐 几何预览").color(theme::TEXT_SECONDARY))
            .on_hover_text("查看当前步骤使用的几何图形").clicked() {
            action.open_geo_preview = true;
        }
    });
    ui.add_space(2.0);
    if ui.button(egui::RichText::new("◈ 图形 API 沙箱").color(theme::BLUE_LIGHT))
        .on_hover_text("交互式创建、组合和预览几何图形").clicked() {
        action.open_shape_sandbox = true;
    }

    ui.add_space(4.0);
    ui.separator();
    ui.add_space(6.0);

    // ── 导出 / 导入 ──
    ui.colored_label(theme::BLUE_LIGHT, "◈ 导出 / 导入");
    ui.add_space(2.0);
    if ui.button(egui::RichText::new("▣ 导出 PNG").color(theme::TEXT_SECONDARY))
        .on_hover_text("将当前世界画面导出为 PNG 图片").clicked() {
        action.export_png = true;
    }
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        if ui.button(egui::RichText::new("□ 导出 .lwd").color(theme::TEXT_SECONDARY))
            .on_hover_text("保存世界快照").clicked() {
            action.export_lwd = true;
        }
        if ui.button(egui::RichText::new("■ 导入 .lwd").color(theme::TEXT_SECONDARY))
            .on_hover_text("从存档恢复世界").clicked() {
            action.import_lwd = true;
        }
    });

    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);

    // ── 缩放 ──
    ui.colored_label(theme::BLUE_LIGHT, "◈ 缩放");
    ui.horizontal(|ui| {
        if ui.button(egui::RichText::new("＋").color(theme::BLUE_LIGHT)).clicked() {
            action.zoom_in = true;
        }
        if ui.button(egui::RichText::new("－").color(theme::PINK_LIGHT)).clicked() {
            action.zoom_out = true;
        }
        if ui.button(egui::RichText::new("↺ 重置").color(theme::TEXT_SECONDARY)).clicked() {
            action.zoom_reset = true;
        }
    });

    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);

    // ── 配置 ──
    ui.colored_label(theme::BLUE_LIGHT, "◈ 配置");
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        if ui.button(egui::RichText::new("◉ 可视化").color(theme::PINK_LIGHT))
            .on_hover_text("环境/层级覆盖色、文字、分界线开关").clicked() {
            action.open_overlay_config = true;
        }
        if ui.button(egui::RichText::new("▧ 层级").color(theme::BLUE_LIGHT))
            .on_hover_text("编辑层级垂直分布").clicked() {
            action.open_layer_config = true;
        }
    });
    ui.horizontal(|ui| {
        if ui.button(egui::RichText::new("⚙ 性能").color(theme::BLUE_LIGHT))
            .on_hover_text("引擎性能调优 / 生成日志").clicked() {
            action.open_perf_panel = true;
        }
    });

    }); // end ScrollArea

    action
}
