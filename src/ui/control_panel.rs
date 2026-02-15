use egui::{ProgressBar, Ui};

#[derive(Debug, Clone)]
pub struct ControlAction {
    pub zoom_in: bool,
    pub zoom_out: bool,
    pub zoom_reset: bool,
}

impl ControlAction {
    pub fn none() -> Self {
        Self {
            zoom_in: false,
            zoom_out: false,
            zoom_reset: false,
        }
    }
}

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

pub fn show_control_panel(
    ui: &mut Ui,
    world_size: &mut WorldSizeSelection,
    current_step: usize,
    total_steps: usize,
) -> ControlAction {
    ui.heading("控制面板");
    ui.separator();

    ui.label("世界尺寸");
    ui.radio_value(world_size, WorldSizeSelection::Small, "小 (4200 x 1200)");
    ui.radio_value(world_size, WorldSizeSelection::Medium, "中 (6400 x 1800)");
    ui.radio_value(world_size, WorldSizeSelection::Large, "大 (8400 x 2400)");

    ui.separator();
    ui.label("生成进度");
    let progress = if total_steps == 0 {
        0.0
    } else {
        current_step as f32 / total_steps as f32
    };
    ui.add(ProgressBar::new(progress).show_percentage());
    ui.label(format!("步骤: {current_step}/{total_steps}"));

    ui.separator();
    ui.horizontal(|ui| {
        ui.add_enabled(false, egui::Button::new("◀ 上一步"));
        ui.add_enabled(false, egui::Button::new("▶ 下一步"));
        ui.add_enabled(false, egui::Button::new("⏸ 暂停"));
    });

    ui.separator();
    ui.label("当前步骤");
    ui.label("✓ Reset");
    ui.label("→ Terrain");
    ui.label("  Dunes");
    ui.label("  ...");

    ui.separator();
    ui.add_enabled(false, egui::Button::new("📸 导出 PNG"));
    ui.add_enabled(false, egui::Button::new("🔄 重新生成"));

    ui.separator();
    ui.label("缩放");
    let mut action = ControlAction::none();
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
