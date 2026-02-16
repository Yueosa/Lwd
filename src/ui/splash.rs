//! # Splash Screen
//!
//! 在未开始生成时，中央画布上显示 ASCII 字符画。

use egui::{Color32, FontId, Pos2, Rect, Ui, Vec2};
use crate::ui::theme;

/// ASCII 字符画
const ASCII_ART: &str = "\
██╗      ██╗ █████╗ ███╗   ██╗██╗    ██╗ ██████╗ ██████╗ ██╗     ██████╗ 
██║      ██║██╔══██╗████╗  ██║██║    ██║██╔═══██╗██╔══██╗██║     ██╔══██╗
██║      ██║███████║██╔██╗ ██║██║ █╗ ██║██║   ██║██████╔╝██║     ██║  ██║
██║      ██║██╔══██║██║╚██╗██║██║███╗██║██║   ██║██╔══██╗██║     ██║  ██║
███████╗ ██║██║  ██║██║ ╚████║╚███╔███╔╝╚██████╔╝██║  ██║███████╗██████╔╝
╚══════╝ ╚═╝╚═╝  ╚═╝╚═╝  ╚═══╝ ╚══╝╚══╝  ╚═════╝ ╚═╝  ╚═╝╚══════╝╚═════╝";

const SUBTITLE: &str = "Terraria-style 2D World Generation Visualizer";
const HINT: &str = "< 在左侧面板点击 [>> 重新初始化] 开始 >";

/// 在中央画布区域绘制 splash 字符画
pub fn show_splash(ui: &mut Ui) {
    let available = ui.available_size();
    let (rect, _response) = ui.allocate_exact_size(available, egui::Sense::hover());
    let painter = ui.painter_at(rect);

    // 深色背景
    painter.rect_filled(rect, 0.0, theme::BG_DARK);

    let lines: Vec<&str> = ASCII_ART.lines().collect();
    let line_count = lines.len();

    // 用等宽字体绘制字符画
    let mono_font = FontId::monospace(14.0);
    let subtitle_font = FontId::proportional(16.0);
    let hint_font = FontId::proportional(13.0);

    // 计算整体内容高度：字符画 + 间距 + 副标题 + 间距 + 提示
    let line_height = 16.0_f32;
    let art_height = line_count as f32 * line_height;
    let total_height = art_height + 40.0 + 20.0 + 30.0 + 20.0;

    let start_y = rect.center().y - total_height / 2.0;

    // 绘制字符画 — 粉蓝渐变
    for (i, line) in lines.iter().enumerate() {
        let t = i as f32 / (line_count.max(1) - 1) as f32;
        let color = theme::progress_color(t);
        let y = start_y + i as f32 * line_height;
        painter.text(
            Pos2::new(rect.center().x, y),
            egui::Align2::CENTER_CENTER,
            line,
            mono_font.clone(),
            color,
        );
    }

    // 副标题
    let subtitle_y = start_y + art_height + 40.0;
    painter.text(
        Pos2::new(rect.center().x, subtitle_y),
        egui::Align2::CENTER_CENTER,
        SUBTITLE,
        subtitle_font,
        theme::TEXT_SECONDARY,
    );

    // 操作提示 — 闪烁效果（通过 alpha 值周期变化）
    let hint_y = subtitle_y + 50.0;
    let time = ui.ctx().input(|i| i.time);
    let alpha = ((time * 2.0).sin() * 0.3 + 0.7) as f32;
    let hint_color = Color32::from_rgba_unmultiplied(
        theme::PINK.r(),
        theme::PINK.g(),
        theme::PINK.b(),
        (alpha * 255.0) as u8,
    );
    painter.text(
        Pos2::new(rect.center().x, hint_y),
        egui::Align2::CENTER_CENTER,
        HINT,
        hint_font,
        hint_color,
    );

    // 请求重绘以实现闪烁动画
    ui.ctx().request_repaint();
}
