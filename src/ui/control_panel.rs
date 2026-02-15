use egui::{ProgressBar, ScrollArea, Ui};

use crate::generation::{StepInfo, StepStatus};

// ── action returned to the app ──────────────────────────────

#[derive(Debug, Clone)]
pub struct ControlAction {
    pub zoom_in: bool,
    pub zoom_out: bool,
    pub zoom_reset: bool,
    pub step_forward: bool,
    pub step_backward: bool,
    pub regenerate: bool,
}

impl ControlAction {
    pub fn none() -> Self {
        Self {
            zoom_in: false,
            zoom_out: false,
            zoom_reset: false,
            step_forward: false,
            step_backward: false,
            regenerate: false,
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
    step_info: &[StepInfo],
    executed: usize,
    total: usize,
) -> ControlAction {
    let mut action = ControlAction::none();

    ui.heading("控制面板");
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
    ui.label(format!("步骤: {executed}/{total}"));

    ui.separator();

    // ── step controls ──
    ui.horizontal(|ui| {
        if ui
            .add_enabled(executed > 0, egui::Button::new("◀ 上一步"))
            .clicked()
        {
            action.step_backward = true;
        }
        if ui
            .add_enabled(executed < total, egui::Button::new("▶ 下一步"))
            .clicked()
        {
            action.step_forward = true;
        }
    });

    ui.separator();

    // ── step list ──
    ui.label("步骤列表");
    ScrollArea::vertical()
        .max_height(250.0)
        .show(ui, |ui| {
            for (i, info) in step_info.iter().enumerate() {
                let (prefix, color) = match info.status {
                    StepStatus::Completed => ("✓", egui::Color32::from_rgb(100, 200, 100)),
                    StepStatus::Current => ("→", egui::Color32::from_rgb(100, 180, 255)),
                    StepStatus::Pending => ("  ", egui::Color32::from_gray(120)),
                };
                let label = format!("{prefix} {}. {}", i + 1, info.name);
                let resp = ui.colored_label(color, &label);
                if resp.hovered() {
                    resp.on_hover_text(&info.description);
                }
            }
        });

    ui.separator();

    // ── actions ──
    ui.horizontal(|ui| {
        if ui.button("🔄 重新生成").clicked() {
            action.regenerate = true;
        }
        ui.add_enabled(false, egui::Button::new("📸 导出 PNG"));
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

    action
}
