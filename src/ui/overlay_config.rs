/// 可视化覆盖层的 4 个独立开关
#[derive(Debug, Clone, Copy)]
pub struct OverlaySettings {
    /// 环境半透明覆盖色
    pub show_biome_color: bool,
    /// 环境名称文字标签
    pub show_biome_labels: bool,
    /// 层级分界线
    pub show_layer_lines: bool,
    /// 层级名称文字标签
    pub show_layer_labels: bool,
}

impl Default for OverlaySettings {
    fn default() -> Self {
        Self {
            show_biome_color: false,
            show_biome_labels: false,
            show_layer_lines: true,
            show_layer_labels: true,
        }
    }
}

/// 显示可视化配置弹窗。返回 `true` 表示有开关被修改。
pub fn show_overlay_config_window(
    ctx: &egui::Context,
    open: &mut bool,
    settings: &mut OverlaySettings,
) -> bool {
    let mut changed = false;

    egui::Window::new("👁 可视化配置")
        .open(open)
        .resizable(false)
        .default_width(240.0)
        .show(ctx, |ui| {
            ui.label("环境 (Biome)");
            ui.indent("biome_group", |ui| {
                if ui.checkbox(&mut settings.show_biome_color, "显示环境覆盖色").changed() {
                    changed = true;
                }
                if ui.checkbox(&mut settings.show_biome_labels, "显示环境文字标签").changed() {
                    changed = true;
                }
            });

            ui.separator();

            ui.label("层级 (Layer)");
            ui.indent("layer_group", |ui| {
                if ui.checkbox(&mut settings.show_layer_lines, "显示层级分界线").changed() {
                    changed = true;
                }
                if ui.checkbox(&mut settings.show_layer_labels, "显示层级文字标签").changed() {
                    changed = true;
                }
            });

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("全部开启").clicked() {
                    settings.show_biome_color = true;
                    settings.show_biome_labels = true;
                    settings.show_layer_lines = true;
                    settings.show_layer_labels = true;
                    changed = true;
                }
                if ui.button("全部关闭").clicked() {
                    settings.show_biome_color = false;
                    settings.show_biome_labels = false;
                    settings.show_layer_lines = false;
                    settings.show_layer_labels = false;
                    changed = true;
                }
            });
        });

    changed
}
