//! # 粉蓝白主题
//!
//! 定义 Lian World 的粉蓝白配色方案，并提供一键应用到 egui Style 的函数。

use egui::{Color32, Rounding, Stroke, Style, Visuals};

// ═══════════════════════════════════════════════════════════
// 调色板常量
// ═══════════════════════════════════════════════════════════

/// 粉色（主强调色）
pub const PINK: Color32 = Color32::from_rgb(245, 169, 184);
/// 浅粉（hover / 次级）
pub const PINK_LIGHT: Color32 = Color32::from_rgb(255, 200, 210);
/// 深粉（active / pressed）  
pub const PINK_DARK: Color32 = Color32::from_rgb(210, 130, 150);

/// 蓝色（次强调色）
pub const BLUE: Color32 = Color32::from_rgb(91, 206, 250);
/// 浅蓝
pub const BLUE_LIGHT: Color32 = Color32::from_rgb(145, 225, 255);
/// 深蓝
pub const BLUE_DARK: Color32 = Color32::from_rgb(60, 170, 220);

/// 白色
pub const WHITE: Color32 = Color32::from_rgb(255, 255, 255);
/// 淡白（面板背景）
pub const WHITE_SOFT: Color32 = Color32::from_rgb(245, 245, 250);

/// 深色背景
pub const BG_DARK: Color32 = Color32::from_rgb(30, 30, 40);
/// 面板背景
pub const BG_PANEL: Color32 = Color32::from_rgb(38, 38, 52);
/// 窗口背景
pub const BG_WINDOW: Color32 = Color32::from_rgb(42, 42, 58);
/// 控件背景（非激活）
pub const BG_WIDGET: Color32 = Color32::from_rgb(50, 50, 68);
/// 控件背景（hover）
pub const BG_WIDGET_HOVER: Color32 = Color32::from_rgb(62, 62, 82);
/// 控件背景（active）
pub const BG_WIDGET_ACTIVE: Color32 = Color32::from_rgb(75, 75, 100);

/// 文字颜色
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(235, 235, 245);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(180, 180, 200);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(120, 120, 145);

/// 分隔线
pub const SEPARATOR: Color32 = Color32::from_rgb(65, 65, 85);

// ═══════════════════════════════════════════════════════════
// 步骤列表符号（粉蓝白三色）
// ═══════════════════════════════════════════════════════════

/// 已完成 — 粉色 ●
pub const STEP_COMPLETED_SYMBOL: &str = "●";
pub const STEP_COMPLETED_COLOR: Color32 = PINK;

/// 当前 — 蓝色 ◆
pub const STEP_CURRENT_SYMBOL: &str = "◆";
pub const STEP_CURRENT_COLOR: Color32 = BLUE;

/// 待执行 — 白色 ○
pub const STEP_PENDING_SYMBOL: &str = "○";
pub const STEP_PENDING_COLOR: Color32 = TEXT_MUTED;

/// 子步骤已完成 — 粉色 ✦
pub const SUB_COMPLETED_SYMBOL: &str = "✦";
pub const SUB_COMPLETED_COLOR: Color32 = PINK_DARK;

/// 子步骤当前 — 蓝色 ▸
pub const SUB_CURRENT_SYMBOL: &str = "▸";
pub const SUB_CURRENT_COLOR: Color32 = BLUE_LIGHT;

/// 子步骤待执行 — 灰白 ·
pub const SUB_PENDING_SYMBOL: &str = "·";
pub const SUB_PENDING_COLOR: Color32 = TEXT_MUTED;

// ═══════════════════════════════════════════════════════════
// 进度条颜色（渐变：粉 → 蓝）
// ═══════════════════════════════════════════════════════════

/// 根据进度返回插值颜色：0.0 = 粉色, 1.0 = 蓝色
pub fn progress_color(t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgb(
        (PINK.r() as f32 + (BLUE.r() as f32 - PINK.r() as f32) * t) as u8,
        (PINK.g() as f32 + (BLUE.g() as f32 - PINK.g() as f32) * t) as u8,
        (PINK.b() as f32 + (BLUE.b() as f32 - PINK.b() as f32) * t) as u8,
    )
}

// ═══════════════════════════════════════════════════════════
// 应用主题
// ═══════════════════════════════════════════════════════════

/// 将粉蓝白主题应用到 egui context
pub fn apply_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    
    // ── Visuals (暗色基底) ──
    let mut visuals = Visuals::dark();
    
    // 背景色
    visuals.panel_fill = BG_PANEL;
    visuals.window_fill = BG_WINDOW;
    visuals.extreme_bg_color = BG_DARK;
    visuals.faint_bg_color = Color32::from_rgb(45, 45, 60);
    
    // 控件样式
    let rounding = Rounding::same(4.0);
    
    // 非激活 Widget
    visuals.widgets.inactive.bg_fill = BG_WIDGET;
    visuals.widgets.inactive.weak_bg_fill = BG_WIDGET;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(70, 70, 90));
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.inactive.rounding = rounding;
    
    // Hovered Widget
    visuals.widgets.hovered.bg_fill = BG_WIDGET_HOVER;
    visuals.widgets.hovered.weak_bg_fill = BG_WIDGET_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, BLUE);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.5, WHITE);
    visuals.widgets.hovered.rounding = rounding;
    
    // Active (pressed) Widget
    visuals.widgets.active.bg_fill = BG_WIDGET_ACTIVE;
    visuals.widgets.active.weak_bg_fill = BG_WIDGET_ACTIVE;
    visuals.widgets.active.bg_stroke = Stroke::new(1.5, PINK);
    visuals.widgets.active.fg_stroke = Stroke::new(2.0, WHITE);
    visuals.widgets.active.rounding = rounding;
    
    // Open (dropdown/combo 展开)
    visuals.widgets.open.bg_fill = BG_WIDGET_ACTIVE;
    visuals.widgets.open.weak_bg_fill = BG_WIDGET_ACTIVE;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, BLUE_LIGHT);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, WHITE);
    visuals.widgets.open.rounding = rounding;
    
    // 非交互元素（标签等）
    visuals.widgets.noninteractive.bg_fill = BG_PANEL;
    visuals.widgets.noninteractive.weak_bg_fill = BG_PANEL;
    visuals.widgets.noninteractive.bg_stroke = Stroke::NONE;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.noninteractive.rounding = rounding;
    
    // 选中项强调色
    visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(91, 206, 250, 80);
    visuals.selection.stroke = Stroke::new(1.0, BLUE_LIGHT);
    
    // 超链接
    visuals.hyperlink_color = BLUE_LIGHT;
    
    // 窗口边框
    visuals.window_stroke = Stroke::new(1.0, Color32::from_rgb(80, 80, 110));
    visuals.window_rounding = Rounding::same(6.0);
    
    style.visuals = visuals;
    
    // ── Spacing ──
    style.spacing.item_spacing = egui::vec2(6.0, 4.0);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);

    ctx.set_style(style);
}
