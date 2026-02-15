use egui::Ui;

pub fn show_status_bar(ui: &mut Ui, fps: f32, memory_hint_mb: usize, message: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(format!("状态: {message}"));
        ui.separator();
        ui.label(format!("FPS: {:.0}", fps));
        ui.separator();
        ui.label(format!("内存: ~{}MB", memory_hint_mb));
    });
}
