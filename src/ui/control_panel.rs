use egui::{Align, Layout, ProgressBar, ScrollArea, Ui};

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
    pub open_overlay_config: bool,
    pub open_layer_config: bool,
    /// 打开当前步骤的算法配置面板
    pub open_step_config: bool,
    /// 导出 PNG
    pub export_png: bool,
    /// 导出 .lwd 存档
    pub export_lwd: bool,
    /// 导入 .lwd 存档
    pub import_lwd: bool,
    /// 应用手动输入的种子
    pub apply_seed: bool,
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
            export_png: false,
            export_lwd: false,
            import_lwd: false,
            apply_seed: false,
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
    seed_input: &mut String,
    phase_info: &[PhaseInfo],
    executed: usize,
    total: usize,
) -> ControlAction {
    let mut action = ControlAction::none();

    ui.with_layout(Layout::top_down(Align::Center), |ui| {
        ui.heading("🗺 Lian World");
    });
    ui.separator();

    // ── world size ──
    ui.label("世界尺寸");
    ui.radio_value(world_size, WorldSizeSelection::Small, "小 (4200×1200)");
    ui.radio_value(world_size, WorldSizeSelection::Medium, "中 (6400×1800)");
    ui.radio_value(world_size, WorldSizeSelection::Large, "大 (8400×2400)");

    ui.separator();

    // ── seed input ──
    ui.label("种子");
    ui.horizontal(|ui| {
        let text_edit = egui::TextEdit::singleline(seed_input)
            .hint_text("输入种子 (十六进制/十进制)")
            .desired_width(140.0);
        let resp = ui.add(text_edit);
        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            action.apply_seed = true;
        }
        if ui.button("✓").on_hover_text("应用种子并重置到第0步").clicked() {
            action.apply_seed = true;
        }
    });

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

    ui.separator();
    ui.label("导出 / 导入");
    if ui.button("📸 导出 PNG").on_hover_text("将当前世界画面导出为 PNG 图片").clicked() {
        action.export_png = true;
    }
    ui.horizontal(|ui| {
        if ui.button("💾 导出 .lwd").on_hover_text("保存世界快照（可完整复现）").clicked() {
            action.export_lwd = true;
        }
        if ui.button("📂 导入 .lwd").on_hover_text("从存档文件恢复世界").clicked() {
            action.import_lwd = true;
        }
    });

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

    // ── overlay + layer config ──
    ui.label("配置");
    ui.horizontal(|ui| {
        if ui.button("👁 可视化配置").on_hover_text("环境/层级的覆盖色、文字、分界线开关").clicked() {
            action.open_overlay_config = true;
        }
        if ui.button("⚙ 层级配置").on_hover_text("编辑层级垂直分布").clicked() {
            action.open_layer_config = true;
        }
    });

    action
}
