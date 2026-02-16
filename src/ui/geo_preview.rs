//! # 几何预览工具窗口
//!
//! 独立调试窗口，展示当前步骤使用的所有几何图形。
//! 包含：
//! - mini-canvas：在独立坐标系中绘制形状轮廓和填充区域
//! - 形状列表：每条记录的标签、类型、参数、显隐开关
//! - 映射到世界画布的功能（未来阶段）

use egui::{
    Color32, Context, Pos2, Rect as EguiRect, Sense, Stroke, Ui, Vec2,
};

use crate::core::geometry::{ShapeParams, ShapeRecord};

// ═══════════════════════════════════════════════════════════
// 窗口状态
// ═══════════════════════════════════════════════════════════

/// 几何预览窗口的持久状态
pub struct GeoPreviewState {
    /// 形状显隐开关（index → visible），长度随 shapes 动态调整
    pub visibility: Vec<bool>,
    /// 当前选中的形状索引（可选）
    pub selected: Option<usize>,
    /// mini-canvas 的缩放偏移
    pub canvas_zoom: f32,
    pub canvas_offset: Vec2,
}

impl Default for GeoPreviewState {
    fn default() -> Self {
        Self {
            visibility: Vec::new(),
            selected: None,
            canvas_zoom: 1.0,
            canvas_offset: Vec2::ZERO,
        }
    }
}

impl GeoPreviewState {
    /// 同步 visibility 数组长度（新增的默认可见）
    fn sync_visibility(&mut self, count: usize) {
        if self.visibility.len() < count {
            self.visibility.resize(count, true);
        } else if self.visibility.len() > count {
            self.visibility.truncate(count);
        }
    }
}

// ═══════════════════════════════════════════════════════════
// 公共接口
// ═══════════════════════════════════════════════════════════

/// 显示几何预览窗口。
///
/// - `open`: 窗口开关
/// - `step_label`: 当前步骤显示名（如 "1.1 太空/地狱填充"）
/// - `shapes`: 当前步骤的形状记录列表
/// - `state`: 窗口持久状态
/// - `world_size`: (width, height) 世界尺寸，用于坐标映射
pub fn show_geo_preview_window(
    ctx: &Context,
    open: &mut bool,
    step_label: &str,
    shapes: &[ShapeRecord],
    state: &mut GeoPreviewState,
    world_size: (u32, u32),
) {
    state.sync_visibility(shapes.len());

    egui::Window::new(format!("📐 几何预览 — {step_label}"))
        .open(open)
        .resizable(true)
        .default_width(480.0)
        .default_height(520.0)
        .show(ctx, |ui| {
            if shapes.is_empty() {
                ui.label("此步骤没有记录几何形状。");
                return;
            }

            // ── mini-canvas ──
            draw_mini_canvas(ui, shapes, state, world_size);

            ui.separator();

            // ── 形状列表 ──
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    draw_shape_list(ui, shapes, state);
                });

            // ── 选中形状的详细参数 ──
            if let Some(sel) = state.selected {
                if sel < shapes.len() {
                    ui.separator();
                    draw_shape_detail(ui, &shapes[sel]);
                }
            }
        });
}

// ═══════════════════════════════════════════════════════════
// mini-canvas 绘制
// ═══════════════════════════════════════════════════════════

fn draw_mini_canvas(
    ui: &mut Ui,
    shapes: &[ShapeRecord],
    state: &mut GeoPreviewState,
    world_size: (u32, u32),
) {
    let available_width = ui.available_width().max(200.0);
    let canvas_height = 260.0_f32;
    let (response, painter) =
        ui.allocate_painter(Vec2::new(available_width, canvas_height), Sense::click_and_drag());
    let canvas_rect = response.rect;

    // 背景
    painter.rect_filled(canvas_rect, 4.0, Color32::from_rgb(25, 25, 35));

    let (ww, wh) = (world_size.0 as f32, world_size.1 as f32);
    if ww <= 0.0 || wh <= 0.0 {
        return;
    }

    // 处理拖拽平移
    if response.dragged() {
        state.canvas_offset += response.drag_delta();
    }
    // 处理滚轮缩放
    let scroll = ui.input(|i| i.raw_scroll_delta.y);
    if scroll != 0.0 && response.hovered() {
        let factor = if scroll > 0.0 { 1.1 } else { 1.0 / 1.1 };
        state.canvas_zoom = (state.canvas_zoom * factor).clamp(0.2, 10.0);
    }

    // 坐标映射: world → canvas
    let padding = 10.0;
    let inner_w = canvas_rect.width() - padding * 2.0;
    let inner_h = canvas_rect.height() - padding * 2.0;

    // 保持世界宽高比
    let scale_x = inner_w / ww;
    let scale_y = inner_h / wh;
    let base_scale = scale_x.min(scale_y);
    let scale = base_scale * state.canvas_zoom;

    let origin_x = canvas_rect.left() + padding + (inner_w - ww * base_scale) / 2.0
        + state.canvas_offset.x;
    let origin_y = canvas_rect.top() + padding + (inner_h - wh * base_scale) / 2.0
        + state.canvas_offset.y;

    let world_to_canvas = |wx: f32, wy: f32| -> Pos2 {
        Pos2::new(origin_x + wx * scale, origin_y + wy * scale)
    };

    // 绘制世界边界框
    let world_tl = world_to_canvas(0.0, 0.0);
    let world_br = world_to_canvas(ww, wh);
    painter.rect_stroke(
        EguiRect::from_min_max(world_tl, world_br),
        2.0,
        Stroke::new(1.0, Color32::from_rgb(80, 80, 100)),
    );

    // 绘制网格线（每 25% 一条参考线）
    let grid_color = Color32::from_rgba_premultiplied(60, 60, 80, 60);
    for frac in [0.25, 0.5, 0.75] {
        let y = world_to_canvas(0.0, wh * frac).y;
        painter.line_segment(
            [Pos2::new(world_tl.x, y), Pos2::new(world_br.x, y)],
            Stroke::new(0.5, grid_color),
        );
        let x = world_to_canvas(ww * frac, 0.0).x;
        painter.line_segment(
            [Pos2::new(x, world_tl.y), Pos2::new(x, world_br.y)],
            Stroke::new(0.5, grid_color),
        );
    }

    // 绘制每个形状
    for (i, shape) in shapes.iter().enumerate() {
        if i >= state.visibility.len() || !state.visibility[i] {
            continue;
        }
        let is_selected = state.selected == Some(i);
        draw_shape_on_canvas(&painter, shape, is_selected, &world_to_canvas, scale);
    }

    // 点击形状选择
    if response.clicked() {
        if let Some(mouse_pos) = response.interact_pointer_pos() {
            // 反向：canvas → world
            let wx = (mouse_pos.x - origin_x) / scale;
            let wy = (mouse_pos.y - origin_y) / scale;
            // 从后向前检测点击（后绘制的在上层）
            let mut hit = None;
            for (i, shape) in shapes.iter().enumerate().rev() {
                if i < state.visibility.len() && state.visibility[i] {
                    let bb = &shape.bbox;
                    if wx >= bb.x_min as f32
                        && wx <= bb.x_max as f32
                        && wy >= bb.y_min as f32
                        && wy <= bb.y_max as f32
                    {
                        hit = Some(i);
                        break;
                    }
                }
            }
            state.selected = hit;
        }
    }
}

fn draw_shape_on_canvas(
    painter: &egui::Painter,
    shape: &ShapeRecord,
    is_selected: bool,
    world_to_canvas: &dyn Fn(f32, f32) -> Pos2,
    scale: f32,
) {
    let [r, g, b, a] = shape.color;
    let fill_alpha = if is_selected { (a as u16 + 40).min(200) as u8 } else { a };
    let fill_color = Color32::from_rgba_unmultiplied(r, g, b, fill_alpha);
    let stroke_color = if is_selected {
        Color32::from_rgb(255, 255, 100)
    } else {
        Color32::from_rgba_unmultiplied(r, g, b, (a as u16 + 80).min(255) as u8)
    };
    let stroke_width = if is_selected { 2.0 } else { 1.0 };

    match &shape.params {
        ShapeParams::Rect { x0, y0, x1, y1 } => {
            let tl = world_to_canvas(*x0 as f32, *y0 as f32);
            let br = world_to_canvas(*x1 as f32, *y1 as f32);
            let rect = EguiRect::from_min_max(tl, br);
            painter.rect_filled(rect, 0.0, fill_color);
            painter.rect_stroke(rect, 0.0, Stroke::new(stroke_width, stroke_color));
        }
        ShapeParams::Ellipse { cx, cy, rx, ry } => {
            let center = world_to_canvas(*cx as f32, *cy as f32);
            let radius = Vec2::new(*rx as f32 * scale, *ry as f32 * scale);
            painter.add(egui::Shape::ellipse_filled(center, radius, fill_color));
            painter.add(egui::Shape::ellipse_stroke(
                center,
                radius,
                Stroke::new(stroke_width, stroke_color),
            ));
        }
        ShapeParams::Trapezoid { y_top, y_bot, top_x0, top_x1, bot_x0, bot_x1 } => {
            let p0 = world_to_canvas(*top_x0 as f32, *y_top as f32);
            let p1 = world_to_canvas(*top_x1 as f32, *y_top as f32);
            let p2 = world_to_canvas(*bot_x1 as f32, *y_bot as f32);
            let p3 = world_to_canvas(*bot_x0 as f32, *y_bot as f32);
            let points = vec![p0, p1, p2, p3];
            painter.add(egui::Shape::convex_polygon(
                points.clone(),
                fill_color,
                Stroke::new(stroke_width, stroke_color),
            ));
        }
        ShapeParams::Column { x, y_start, y_end } => {
            let top = world_to_canvas(*x as f32, *y_start as f32);
            let bot = world_to_canvas(*x as f32 + 1.0, *y_end as f32);
            let rect = EguiRect::from_min_max(top, bot);
            painter.rect_filled(rect, 0.0, fill_color);
        }
        ShapeParams::Composite { .. } => {
            // 组合形状仅绘制 bbox 虚线轮廓
            let bb = &shape.bbox;
            let tl = world_to_canvas(bb.x_min as f32, bb.y_min as f32);
            let br = world_to_canvas(bb.x_max as f32, bb.y_max as f32);
            painter.rect_stroke(
                EguiRect::from_min_max(tl, br),
                0.0,
                Stroke::new(stroke_width, stroke_color),
            );
        }
    }

    // 标签文字
    let bb = &shape.bbox;
    let label_pos = world_to_canvas(
        (bb.x_min + bb.x_max) as f32 / 2.0,
        bb.y_min as f32,
    );
    let text_color = Color32::from_rgba_unmultiplied(r, g, b, 220);
    painter.text(
        Pos2::new(label_pos.x, label_pos.y - 8.0),
        egui::Align2::CENTER_BOTTOM,
        &shape.label,
        egui::FontId::proportional(10.0),
        text_color,
    );
}

// ═══════════════════════════════════════════════════════════
// 形状列表
// ═══════════════════════════════════════════════════════════

fn draw_shape_list(
    ui: &mut Ui,
    shapes: &[ShapeRecord],
    state: &mut GeoPreviewState,
) {
    ui.strong("形状列表");
    ui.add_space(4.0);

    for (i, shape) in shapes.iter().enumerate() {
        let is_selected = state.selected == Some(i);
        let vis = state.visibility.get_mut(i);
        
        ui.horizontal(|ui| {
            // 显隐 checkbox
            if let Some(v) = vis {
                ui.checkbox(v, "");
            }

            // 颜色标记
            let [r, g, b, _] = shape.color;
            let color = Color32::from_rgb(r, g, b);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(12.0, 12.0), Sense::hover());
            ui.painter().rect_filled(rect, 2.0, color);

            // 标签按钮
            let label_text = format!(
                "{} [{}]",
                shape.label,
                shape.params.kind_label(),
            );
            let btn = ui.selectable_label(is_selected, label_text);
            if btn.clicked() {
                state.selected = if is_selected { None } else { Some(i) };
            }
        });
    }
}

// ═══════════════════════════════════════════════════════════
// 形状详细信息面板
// ═══════════════════════════════════════════════════════════

fn draw_shape_detail(ui: &mut Ui, shape: &ShapeRecord) {
    ui.strong(format!("📋 {} — {}", shape.label, shape.params.kind_label()));
    ui.add_space(4.0);

    egui::Grid::new("shape_detail_grid")
        .num_columns(2)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            ui.label("类型:");
            ui.monospace(shape.params.kind_label());
            ui.end_row();

            ui.label("数学描述:");
            ui.monospace(shape.params.math_description());
            ui.end_row();

            ui.label("包围盒:");
            ui.monospace(format!(
                "[{}, {}] → [{}, {}]",
                shape.bbox.x_min, shape.bbox.y_min,
                shape.bbox.x_max, shape.bbox.y_max,
            ));
            ui.end_row();

            let w = (shape.bbox.x_max - shape.bbox.x_min).max(0);
            let h = (shape.bbox.y_max - shape.bbox.y_min).max(0);
            ui.label("尺寸:");
            ui.monospace(format!("{w} × {h}"));
            ui.end_row();

            // 形状特有参数
            match &shape.params {
                ShapeParams::Rect { x0, y0, x1, y1 } => {
                    ui.label("左上:");
                    ui.monospace(format!("({x0}, {y0})"));
                    ui.end_row();
                    ui.label("右下:");
                    ui.monospace(format!("({x1}, {y1})"));
                    ui.end_row();
                }
                ShapeParams::Ellipse { cx, cy, rx, ry } => {
                    ui.label("中心:");
                    ui.monospace(format!("({cx:.1}, {cy:.1})"));
                    ui.end_row();
                    ui.label("半径:");
                    ui.monospace(format!("rx={rx:.1}, ry={ry:.1}"));
                    ui.end_row();
                }
                ShapeParams::Trapezoid { y_top, y_bot, top_x0, top_x1, bot_x0, bot_x1 } => {
                    ui.label("Y 范围:");
                    ui.monospace(format!("[{y_top}, {y_bot})"));
                    ui.end_row();
                    ui.label("上边:");
                    ui.monospace(format!("[{top_x0:.1}, {top_x1:.1})"));
                    ui.end_row();
                    ui.label("下边:");
                    ui.monospace(format!("[{bot_x0:.1}, {bot_x1:.1})"));
                    ui.end_row();
                }
                ShapeParams::Column { x, y_start, y_end } => {
                    ui.label("位置:");
                    ui.monospace(format!("x={x}, y∈[{y_start}, {y_end})"));
                    ui.end_row();
                }
                ShapeParams::Composite { description } => {
                    ui.label("描述:");
                    ui.monospace(description);
                    ui.end_row();
                }
            }
        });
}
