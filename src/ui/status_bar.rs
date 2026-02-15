use egui::Ui;

pub fn show_status_bar(
    ui: &mut Ui,
    fps: f32,
    memory_hint_mb: usize,
    message: &str,
    hover: &str,
    seed: u64,
    step_progress: &str,
    world_size_label: &str,
) {
    ui.horizontal_wrapped(|ui| {
        ui.label(format!("状态: {message}"));
        if !hover.is_empty() {
            ui.separator();
            ui.label(hover);
        }
        ui.separator();
        ui.label(step_progress);
        ui.separator();
        ui.label(world_size_label);
        ui.separator();
        ui.label(format!("Seed: {:016X}", seed));
        ui.separator();
        ui.label(format!("FPS: {:.0}", fps));
        ui.separator();
        ui.label(format!("内存: ~{}MB", memory_hint_mb));
    });
}
