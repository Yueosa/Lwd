use egui::{ProgressBar, ScrollArea, Ui};

use crate::generation::{PhaseInfo, StepStatus};

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
    pub biome_overlay_toggled: bool,
    pub layer_overlay_toggled: bool,
    pub open_layer_config: bool,
    /// 打开当前步骤的算法配置面板
    pub open_step_config: bool,
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
            biome_overlay_toggled: false,
            layer_overlay_toggled: false,
            open_layer_config: false,
            open_step_config: false,
        }
    }
}

// ── world size enum ─────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldSizeSelection {
    Small,
    Medium,
    Large,
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
    phase_info: &[PhaseInfo],
    executed: usize,
    total: usize,
    show_biome_overlay: &mut bool,
    show_layer_overlay: &mut bool,
) -> ControlAction {
    let mut action = ControlAction::none();

    ui.heading("🗺 Lian World");
    ui.separator();

    // ── world size ──
    ui.label("世界尺寸");
    ui.radio_value(world_size, WorldSizeSelection::Small, "小 (4200×1200)");
    ui.radio_value(world_size, WorldSizeSelection::Medium, "中 (6400×1800)");
    ui.radio_value(world_size, WorldSizeSelection::Large, "大 (8400×2400)");

    ui.separator();

    // ── progress ──
    ui.label("生成进度");
    let progress = if total == 0 {
        0.0
    } else {
        executed as f32 / total as f32
    };
    ui.add(ProgressBar::new(progress).show_percentage());
    ui.label(format!("子步骤: {executed}/{total}"));

    ui.separator();

    // ── step controls (4 buttons) ──
    ui.label("步进控制");
    ui.horizontal(|ui| {
        if ui
            .add_enabled(executed > 0, egui::Button::new("◀◀"))
            .on_hover_text("大步后退 (-1.0 回到阶段开头)")
            .clicked()
        {
            action.step_backward_phase = true;
        }
        if ui
            .add_enabled(executed > 0, egui::Button::new("◀"))
            .on_hover_text("小步后退 (-0.1)")
            .clicked()
        {
            action.step_backward_sub = true;
        }
        if ui
            .add_enabled(executed < total, egui::Button::new("▶"))
            .on_hover_text("小步前进 (+0.1)")
            .clicked()
        {
            action.step_forward_sub = true;
        }
        if ui
            .add_enabled(executed < total, egui::Button::new("▶▶"))
            .on_hover_text("大步前进 (+1.0 执行完当前阶段)")
            .clicked()
        {
            action.step_forward_phase = true;
        }
    });

    ui.separator();

    // ── phase/step list (two-level) ──
    ui.label("步骤列表");
    ScrollArea::vertical()
        .max_height(300.0)
        .show(ui, |ui| {
            for phase in phase_info {
                let (phase_prefix, phase_color) = match phase.status {
                    StepStatus::Completed => ("✓", egui::Color32::from_rgb(100, 200, 100)),
                    StepStatus::Current => ("▶", egui::Color32::from_rgb(100, 180, 255)),
                    StepStatus::Pending => ("  ", egui::Color32::from_gray(120)),
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
                        StepStatus::Completed => ("✓", egui::Color32::from_rgb(80, 170, 80)),
                        StepStatus::Current => ("→", egui::Color32::from_rgb(80, 160, 230)),
                        StepStatus::Pending => ("·", egui::Color32::from_gray(100)),
                    };
                    
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        let sub_label = format!("{sub_prefix} {} {}", sub.display_id, sub.name);
                        let resp = ui.colored_label(sub_color, &sub_label);
                        
                        if resp.hovered() {
                            resp.on_hover_ui(|ui| {
                                ui.label(&sub.description);
                                if let Some(url) = &sub.doc_url {
                                    ui.hyperlink_to("📖 查看算法文档", url);
                                }
                            });
                        }
                    });
                }
            }
        });

    ui.separator();

    // ── actions ──
    ui.label("生成操作");
    ui.horizontal(|ui| {
        if ui.button("🔄 一键生成").on_hover_text("新种子 → 重置 → 执行全部步骤").clicked() {
            action.reset_and_step = true;
            action.run_all = true;
        }
        if ui.button("♻ 重新初始化").on_hover_text("新种子 → 重置到第0步，可手动步进").clicked() {
            action.reset_and_step = true;
        }
    });
    if ui
        .add_enabled(executed < total, egui::Button::new("⏩ 执行到底"))
        .on_hover_text("从当前步骤一直执行到最后")
        .clicked()
    {
        action.run_all = true;
    }
    
    if ui.button("⚙ 当前步骤算法").on_hover_text("打开当前步骤的算法参数配置面板").clicked() {
        action.open_step_config = true;
    }
    
    ui.add_enabled(false, egui::Button::new("📸 导出 PNG"));

    ui.separator();

    // ── zoom ──
    ui.label("缩放");
    ui.horizontal(|ui| {
        if ui.button("+").clicked() {
            action.zoom_in = true;
        }
        if ui.button("-").clicked() {
            action.zoom_out = true;
        }
        if ui.button("重置").clicked() {
            action.zoom_reset = true;
        }
    });

    ui.separator();

    // ── overlay ──
    ui.label("可视化图层");
    if ui.checkbox(show_biome_overlay, "显示环境划分").changed() {
        action.biome_overlay_toggled = true;
    }
    if ui.checkbox(show_layer_overlay, "显示层级划分").changed() {
        action.layer_overlay_toggled = true;
    }
    
    // 层级配置按钮
    if ui.button("⚙ 配置层级").clicked() {
        action.open_layer_config = true;
    }

    action
}
