mod algorithms;
mod config;
mod core;
mod generation;
mod rendering;
mod storage;
mod ui;

use ui::app::LianWorldApp;

fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Lian Terraria World Generator")
            .with_inner_size([1400.0, 860.0])
            .with_app_id("lian-world"),
        ..Default::default()
    };

    eframe::run_native(
        "Lian Terraria World Generator",
        options,
        Box::new(|cc| Box::new(LianWorldApp::new(cc))),
    )
    .expect("窗口启动失败");
}
