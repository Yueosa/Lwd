//! # 图形 API 沙箱窗口
//!
//! 交互式窗口，用于创建、预览和组合几何图形 API。
//! 功能：
//! - 添加基础形状（矩形 / 椭圆 / 梯形 / 列）
//! - 调整每种形状的参数（滑块 + 数值拖放）
//! - 使用集合运算组合形状（并集 / 交集 / 差集）
//! - 实时 mini-canvas 预览组合结果
//! - 显示数学描述 + 代码片段

use egui::{
    Color32, Context, Pos2, Rect as EguiRect, Sense, Stroke, Ui, Vec2,
};

use crate::core::geometry::{
    BoundingBox, Column, Ellipse, Rect, Shape, ShapeKind, Trapezoid,
};

// ═══════════════════════════════════════════════════════════
// 数据结构
// ═══════════════════════════════════════════════════════════

/// 沙箱中一个形状条目
#[derive(Debug, Clone)]
pub struct SandboxShape {
    /// 形状类型
    pub kind: ShapeKind,
    /// 自定义标签
    pub label: String,
    /// 颜色（预览用）
    pub color: [u8; 4],
    /// 是否可见
    pub visible: bool,
    // ── 各类型参数 ──
    // 矩形
    pub rect_x0: i32,
    pub rect_y0: i32,
    pub rect_x1: i32,
    pub rect_y1: i32,
    // 椭圆
    pub ell_cx: f64,
    pub ell_cy: f64,
    pub ell_rx: f64,
    pub ell_ry: f64,
    // 梯形
    pub trap_y_top: i32,
    pub trap_y_bot: i32,
    pub trap_top_x0: f64,
    pub trap_top_x1: f64,
    pub trap_bot_x0: f64,
    pub trap_bot_x1: f64,
    // 列
    pub col_x: i32,
    pub col_y_start: i32,
    pub col_y_end: i32,
}

impl SandboxShape {
    /// 创建默认形状（居中放置）
    fn new_default(kind: ShapeKind, index: usize, world_w: u32, world_h: u32) -> Self {
        let cx = world_w as f64 / 2.0;
        let cy = world_h as f64 / 2.0;
        let w4 = (world_w / 4) as i32;
        let h4 = (world_h / 4) as i32;

        // 每个形状给不同色
        let palette: [[u8; 4]; 6] = [
            [100, 180, 255, 80],
            [255, 130, 100, 80],
            [100, 230, 140, 80],
            [220, 180, 60, 80],
            [180, 120, 255, 80],
            [255, 180, 200, 80],
        ];
        let color = palette[index % palette.len()];

        Self {
            kind,
            label: format!("形状 {}", index + 1),
            color,
            visible: true,
            // rect defaults → center quarter
            rect_x0: cx as i32 - w4,
            rect_y0: cy as i32 - h4,
            rect_x1: cx as i32 + w4,
            rect_y1: cy as i32 + h4,
            // ellipse
            ell_cx: cx,
            ell_cy: cy,
            ell_rx: w4 as f64,
            ell_ry: h4 as f64,
            // trapezoid
            trap_y_top: cy as i32 - h4,
            trap_y_bot: cy as i32 + h4,
            trap_top_x0: cx - w4 as f64 * 0.6,
            trap_top_x1: cx + w4 as f64 * 0.6,
            trap_bot_x0: cx - w4 as f64,
            trap_bot_x1: cx + w4 as f64,
            // column
            col_x: cx as i32,
            col_y_start: cy as i32 - h4,
            col_y_end: cy as i32 + h4,
        }
    }

    /// 按当前 kind 构造对应的 BoundingBox
    fn bounding_box(&self) -> BoundingBox {
        match self.kind {
            ShapeKind::Rect => {
                let r = Rect::new(self.rect_x0, self.rect_y0, self.rect_x1, self.rect_y1);
                r.bounding_box()
            }
            ShapeKind::Ellipse => {
                let e = Ellipse::new(self.ell_cx, self.ell_cy, self.ell_rx, self.ell_ry);
                e.bounding_box()
            }
            ShapeKind::Trapezoid => {
                let t = Trapezoid::new(
                    self.trap_y_top, self.trap_y_bot,
                    self.trap_top_x0, self.trap_top_x1,
                    self.trap_bot_x0, self.trap_bot_x1,
                );
                t.bounding_box()
            }
            ShapeKind::Column => {
                Column::new(self.col_x, self.col_y_start, self.col_y_end).bounding_box()
            }
        }
    }

    /// 点检测
    fn contains(&self, x: i32, y: i32) -> bool {
        match self.kind {
            ShapeKind::Rect => {
                Rect::new(self.rect_x0, self.rect_y0, self.rect_x1, self.rect_y1)
                    .contains(x, y)
            }
            ShapeKind::Ellipse => {
                Ellipse::new(self.ell_cx, self.ell_cy, self.ell_rx, self.ell_ry)
                    .contains(x, y)
            }
            ShapeKind::Trapezoid => {
                Trapezoid::new(
                    self.trap_y_top, self.trap_y_bot,
                    self.trap_top_x0, self.trap_top_x1,
                    self.trap_bot_x0, self.trap_bot_x1,
                ).contains(x, y)
            }
            ShapeKind::Column => {
                Column::new(self.col_x, self.col_y_start, self.col_y_end).contains(x, y)
            }
        }
    }

    /// 数学描述
    fn math_desc(&self) -> String {
        match self.kind {
            ShapeKind::Rect => format!(
                "x∈[{},{}), y∈[{},{})",
                self.rect_x0, self.rect_x1, self.rect_y0, self.rect_y1
            ),
            ShapeKind::Ellipse => format!(
                "(x-{:.0})²/{:.0}² + (y-{:.0})²/{:.0}² ≤ 1",
                self.ell_cx, self.ell_rx, self.ell_cy, self.ell_ry
            ),
            ShapeKind::Trapezoid => format!(
                "y∈[{},{}), 上[{:.0},{:.0}) 下[{:.0},{:.0})",
                self.trap_y_top, self.trap_y_bot,
                self.trap_top_x0, self.trap_top_x1,
                self.trap_bot_x0, self.trap_bot_x1,
            ),
            ShapeKind::Column => format!(
                "x={}, y∈[{},{})",
                self.col_x, self.col_y_start, self.col_y_end
            ),
        }
    }

    /// 生成 Rust 代码片段
    fn code_snippet(&self) -> String {
        match self.kind {
            ShapeKind::Rect => format!(
                "let shape = Rect::new({}, {}, {}, {});",
                self.rect_x0, self.rect_y0, self.rect_x1, self.rect_y1
            ),
            ShapeKind::Ellipse => format!(
                "let shape = Ellipse::new({:.1}, {:.1}, {:.1}, {:.1});",
                self.ell_cx, self.ell_cy, self.ell_rx, self.ell_ry
            ),
            ShapeKind::Trapezoid => format!(
                "let shape = Trapezoid::new({}, {}, {:.1}, {:.1}, {:.1}, {:.1});",
                self.trap_y_top, self.trap_y_bot,
                self.trap_top_x0, self.trap_top_x1,
                self.trap_bot_x0, self.trap_bot_x1,
            ),
            ShapeKind::Column => format!(
                "let shape = Column::new({}, {}, {});",
                self.col_x, self.col_y_start, self.col_y_end
            ),
        }
    }
}

/// 集合运算类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetOp {
    Union,
    Intersect,
    Subtract,
}

impl SetOp {
    pub fn label(self) -> &'static str {
        match self {
            SetOp::Union => "∪ 并集",
            SetOp::Intersect => "∩ 交集",
            SetOp::Subtract => "− 差集",
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            SetOp::Union => "∪",
            SetOp::Intersect => "∩",
            SetOp::Subtract => "−",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            SetOp::Union => ".union",
            SetOp::Intersect => ".intersect",
            SetOp::Subtract => ".subtract",
        }
    }
}

/// 组合表达式节点
#[derive(Debug, Clone)]
pub struct CombineNode {
    /// 左操作数（形状列表索引）
    pub left: usize,
    /// 运算类型
    pub op: SetOp,
    /// 右操作数（形状列表索引）
    pub right: usize,
    /// 结果颜色
    pub color: [u8; 4],
    /// 是否可见
    pub visible: bool,
}

/// 沙箱窗口持久状态
pub struct ShapeSandboxState {
    /// 基础形状列表
    pub shapes: Vec<SandboxShape>,
    /// 组合运算列表
    pub combines: Vec<CombineNode>,
    /// 选中的形状索引（基础形状）
    pub selected_shape: Option<usize>,
    /// 选中的组合索引
    pub selected_combine: Option<usize>,
    /// 新形状类型选择器
    pub new_shape_kind: ShapeKind,
    /// mini-canvas 缩放
    pub canvas_zoom: f32,
    /// mini-canvas 偏移
    pub canvas_offset: Vec2,
    /// 新组合 — 左操作数索引
    pub new_combine_left: usize,
    /// 新组合 — 右操作数索引
    pub new_combine_right: usize,
    /// 新组合 — 运算类型
    pub new_combine_op: SetOp,
    /// 显示模式：0=仅基础, 1=仅组合, 2=全部
    pub display_mode: u8,
}

impl Default for ShapeSandboxState {
    fn default() -> Self {
        Self {
            shapes: Vec::new(),
            combines: Vec::new(),
            selected_shape: None,
            selected_combine: None,
            new_shape_kind: ShapeKind::Rect,
            canvas_zoom: 1.0,
            canvas_offset: Vec2::ZERO,
            new_combine_left: 0,
            new_combine_right: 0,
            new_combine_op: SetOp::Union,
            display_mode: 2,
        }
    }
}

// ═══════════════════════════════════════════════════════════
// 公共接口
// ═══════════════════════════════════════════════════════════

/// 显示图形 API 沙箱窗口。
pub fn show_shape_sandbox_window(
    ctx: &Context,
    open: &mut bool,
    state: &mut ShapeSandboxState,
    world_size: (u32, u32),
) {
    egui::Window::new("◈ 图形 API 沙箱")
        .open(open)
        .resizable(true)
        .default_width(600.0)
        .default_height(640.0)
        .show(ctx, |ui| {
            // ── 顶部工具栏 ──
            draw_toolbar(ui, state, world_size);

            ui.separator();

            // ── 主体区域：左形状列表 + 右画布 ──
            let available = ui.available_size();
            let list_width = 220.0_f32.min(available.x * 0.4);

            ui.horizontal(|ui| {
                // 左侧：形状列表 + 组合列表
                ui.vertical(|ui| {
                    ui.set_width(list_width);
                    egui::ScrollArea::vertical()
                        .max_height(available.y - 30.0)
                        .show(ui, |ui| {
                            draw_shape_list(ui, state);
                            ui.separator();
                            draw_combine_list(ui, state);
                        });
                });

                ui.separator();

                // 右侧：画布 + 详情
                ui.vertical(|ui| {
                    draw_sandbox_canvas(ui, state, world_size);
                    ui.separator();
                    draw_detail_panel(ui, state);
                });
            });
        });
}

// ═══════════════════════════════════════════════════════════
// 顶部工具栏
// ═══════════════════════════════════════════════════════════

fn draw_toolbar(ui: &mut Ui, state: &mut ShapeSandboxState, world_size: (u32, u32)) {
    ui.horizontal(|ui| {
        ui.label("添加形状:");

        // 形状类型选择
        egui::ComboBox::from_id_source("sandbox_shape_kind")
            .selected_text(state.new_shape_kind.display_name())
            .width(70.0)
            .show_ui(ui, |ui: &mut Ui| {
                for kind in ShapeKind::all() {
                    ui.selectable_value(&mut state.new_shape_kind, *kind, kind.display_name());
                }
            });

        if ui.button("➕ 添加").clicked() {
            let idx = state.shapes.len();
            state.shapes.push(SandboxShape::new_default(
                state.new_shape_kind,
                idx,
                world_size.0,
                world_size.1,
            ));
            state.selected_shape = Some(idx);
            state.selected_combine = None;
        }

        ui.separator();

        // 显示模式
        ui.label("显示:");
        let mode_labels = ["基础", "组合", "全部"];
        for (i, label) in mode_labels.iter().enumerate() {
            if ui.selectable_label(state.display_mode == i as u8, *label).clicked() {
                state.display_mode = i as u8;
            }
        }

        ui.separator();

        // 重置缩放
        if ui.button("⟳ 复位").clicked() {
            state.canvas_zoom = 1.0;
            state.canvas_offset = Vec2::ZERO;
        }
    });
}

// ═══════════════════════════════════════════════════════════
// 形状列表
// ═══════════════════════════════════════════════════════════

fn draw_shape_list(ui: &mut Ui, state: &mut ShapeSandboxState) {
    ui.strong("基础形状");
    ui.add_space(4.0);

    if state.shapes.is_empty() {
        ui.weak("（空）使用上方工具栏添加形状");
        return;
    }

    let mut to_remove: Option<usize> = None;

    for i in 0..state.shapes.len() {
        let is_sel = state.selected_shape == Some(i);
        let [r, g, b, _] = state.shapes[i].color;
        let label_text = format!(
            "[{}] {} — {}",
            i, state.shapes[i].label, state.shapes[i].kind.display_name()
        );

        ui.horizontal(|ui| {
            // 颜色块
            let (rect, _) = ui.allocate_exact_size(Vec2::new(10.0, 10.0), Sense::hover());
            ui.painter().rect_filled(rect, 2.0, Color32::from_rgb(r, g, b));

            // 可见性
            let mut vis = state.shapes[i].visible;
            if ui.checkbox(&mut vis, "").changed() {
                state.shapes[i].visible = vis;
            }

            // 标签
            let btn = ui.selectable_label(is_sel, &label_text);
            if btn.clicked() {
                state.selected_shape = if is_sel { None } else { Some(i) };
                state.selected_combine = None;
            }

            // 删除
            if ui.small_button("✕").clicked() {
                to_remove = Some(i);
            }
        });
    }

    if let Some(idx) = to_remove {
        state.shapes.remove(idx);
        // 修正组合引用
        state.combines.retain(|c| c.left != idx && c.right != idx);
        for c in &mut state.combines {
            if c.left > idx { c.left -= 1; }
            if c.right > idx { c.right -= 1; }
        }
        if state.selected_shape == Some(idx) {
            state.selected_shape = None;
        } else if let Some(s) = state.selected_shape {
            if s > idx { state.selected_shape = Some(s - 1); }
        }
    }
}

// ═══════════════════════════════════════════════════════════
// 组合运算列表
// ═══════════════════════════════════════════════════════════

fn draw_combine_list(ui: &mut Ui, state: &mut ShapeSandboxState) {
    ui.strong("集合运算");
    ui.add_space(4.0);

    let shape_count = state.shapes.len();
    if shape_count < 2 {
        ui.weak("需要至少两个基础形状才能创建组合");
        return;
    }

    // 新组合输入区
    ui.horizontal(|ui| {
        // 左操作数
        let left_label = if state.new_combine_left < shape_count {
            format!("[{}]", state.new_combine_left)
        } else {
            "?".to_string()
        };
        egui::ComboBox::from_id_source("comb_left")
            .selected_text(&left_label)
            .width(40.0)
            .show_ui(ui, |ui| {
                for i in 0..shape_count {
                    ui.selectable_value(
                        &mut state.new_combine_left,
                        i,
                        format!("[{}] {}", i, &state.shapes[i].label),
                    );
                }
            });

        // 运算符
        egui::ComboBox::from_id_source("comb_op")
            .selected_text(state.new_combine_op.symbol())
            .width(50.0)
            .show_ui(ui, |ui| {
                for op in [SetOp::Union, SetOp::Intersect, SetOp::Subtract] {
                    ui.selectable_value(&mut state.new_combine_op, op, op.label());
                }
            });

        // 右操作数
        let right_label = if state.new_combine_right < shape_count {
            format!("[{}]", state.new_combine_right)
        } else {
            "?".to_string()
        };
        egui::ComboBox::from_id_source("comb_right")
            .selected_text(&right_label)
            .width(40.0)
            .show_ui(ui, |ui| {
                for i in 0..shape_count {
                    ui.selectable_value(
                        &mut state.new_combine_right,
                        i,
                        format!("[{}] {}", i, &state.shapes[i].label),
                    );
                }
            });

        if ui.button("➕").clicked()
            && state.new_combine_left < shape_count
            && state.new_combine_right < shape_count
        {
            state.combines.push(CombineNode {
                left: state.new_combine_left,
                op: state.new_combine_op,
                right: state.new_combine_right,
                color: [255, 220, 100, 90],
                visible: true,
            });
        }
    });

    ui.add_space(4.0);

    // 已有组合列表
    let mut to_remove: Option<usize> = None;
    for ci in 0..state.combines.len() {
        let is_sel = state.selected_combine == Some(ci);
        let left_idx = state.combines[ci].left;
        let right_idx = state.combines[ci].right;
        let op = state.combines[ci].op;

        let left_name = state.shapes.get(left_idx)
            .map(|s| s.label.clone())
            .unwrap_or_else(|| "?".to_string());
        let right_name = state.shapes.get(right_idx)
            .map(|s| s.label.clone())
            .unwrap_or_else(|| "?".to_string());
        let txt = format!("[{}] {} [{}] = C{}", left_idx, op.symbol(), right_idx, ci);
        let hover = format!("{} {} {}", left_name, op.symbol(), right_name);

        ui.horizontal(|ui| {
            let mut vis = state.combines[ci].visible;
            if ui.checkbox(&mut vis, "").changed() {
                state.combines[ci].visible = vis;
            }

            let btn = ui.selectable_label(is_sel, &txt);
            if btn.clicked() {
                state.selected_combine = if is_sel { None } else { Some(ci) };
                state.selected_shape = None;
            }

            btn.on_hover_text(&hover);

            if ui.small_button("✕").clicked() {
                to_remove = Some(ci);
            }
        });
    }

    if let Some(idx) = to_remove {
        state.combines.remove(idx);
        if state.selected_combine == Some(idx) {
            state.selected_combine = None;
        } else if let Some(c) = state.selected_combine {
            if c > idx { state.selected_combine = Some(c - 1); }
        }
    }
}

// ═══════════════════════════════════════════════════════════
// 沙箱画布
// ═══════════════════════════════════════════════════════════

fn draw_sandbox_canvas(
    ui: &mut Ui,
    state: &mut ShapeSandboxState,
    world_size: (u32, u32),
) {
    let available_width = ui.available_width().max(200.0);
    let canvas_height = 300.0_f32;
    let (response, painter) =
        ui.allocate_painter(Vec2::new(available_width, canvas_height), Sense::click_and_drag());
    let canvas_rect = response.rect;

    // 背景
    painter.rect_filled(canvas_rect, 4.0, Color32::from_rgb(20, 20, 30));

    let (ww, wh) = (world_size.0 as f32, world_size.1 as f32);
    if ww <= 0.0 || wh <= 0.0 {
        return;
    }

    // 拖拽平移
    if response.dragged() {
        state.canvas_offset += response.drag_delta();
    }
    // 滚轮缩放
    let scroll = ui.input(|i| i.raw_scroll_delta.y);
    if scroll != 0.0 && response.hovered() {
        let factor = if scroll > 0.0 { 1.1 } else { 1.0 / 1.1 };
        state.canvas_zoom = (state.canvas_zoom * factor).clamp(0.1, 20.0);
    }

    // 坐标映射
    let padding = 8.0;
    let inner_w = canvas_rect.width() - padding * 2.0;
    let inner_h = canvas_rect.height() - padding * 2.0;
    let scale_x = inner_w / ww;
    let scale_y = inner_h / wh;
    let base_scale = scale_x.min(scale_y);
    let scale = base_scale * state.canvas_zoom;

    let origin_x = canvas_rect.left() + padding + (inner_w - ww * base_scale) / 2.0
        + state.canvas_offset.x;
    let origin_y = canvas_rect.top() + padding + (inner_h - wh * base_scale) / 2.0
        + state.canvas_offset.y;

    let w2c = |wx: f32, wy: f32| -> Pos2 {
        Pos2::new(origin_x + wx * scale, origin_y + wy * scale)
    };

    // 世界边框
    let world_tl = w2c(0.0, 0.0);
    let world_br = w2c(ww, wh);
    painter.rect_stroke(
        EguiRect::from_min_max(world_tl, world_br),
        2.0,
        Stroke::new(1.0, Color32::from_rgb(60, 60, 80)),
    );

    // 参考网格（25%）
    let grid_color = Color32::from_rgba_premultiplied(50, 50, 70, 50);
    for frac in [0.25, 0.5, 0.75] {
        let y = w2c(0.0, wh * frac).y;
        painter.line_segment(
            [Pos2::new(world_tl.x, y), Pos2::new(world_br.x, y)],
            Stroke::new(0.5, grid_color),
        );
        let x = w2c(ww * frac, 0.0).x;
        painter.line_segment(
            [Pos2::new(x, world_tl.y), Pos2::new(x, world_br.y)],
            Stroke::new(0.5, grid_color),
        );
    }

    // ── 绘制基础形状 ──
    let show_base = state.display_mode == 0 || state.display_mode == 2;
    let show_combines = state.display_mode == 1 || state.display_mode == 2;

    if show_base {
        for (i, shape) in state.shapes.iter().enumerate() {
            if !shape.visible {
                continue;
            }
            let is_sel = state.selected_shape == Some(i);
            draw_sandbox_shape(&painter, shape, is_sel, &w2c, scale);
        }
    }

    // ── 绘制组合结果（像素级采样）──
    if show_combines {
        for (ci, comb) in state.combines.iter().enumerate() {
            if !comb.visible {
                continue;
            }
            let left = state.shapes.get(comb.left);
            let right = state.shapes.get(comb.right);
            if left.is_none() || right.is_none() {
                continue;
            }
            let left = left.unwrap();
            let right = right.unwrap();

            let is_sel = state.selected_combine == Some(ci);
            draw_combine_result(&painter, left, right, comb, is_sel, &w2c, scale, world_size);
        }
    }

    // 坐标提示
    if response.hovered() {
        if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
            let wx = ((pos.x - origin_x) / scale) as i32;
            let wy = ((pos.y - origin_y) / scale) as i32;
            egui::show_tooltip_at_pointer(ui.ctx(), ui.id().with("coord_tip"), |ui| {
                ui.monospace(format!("({wx}, {wy})"));
            });
        }
    }

    // 点击选择基础形状
    if response.clicked() {
        if let Some(mpos) = response.interact_pointer_pos() {
            let wx = (mpos.x - origin_x) / scale;
            let wy = (mpos.y - origin_y) / scale;
            let mut hit = None;
            for (i, shape) in state.shapes.iter().enumerate().rev() {
                if shape.visible {
                    let bb = shape.bounding_box();
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
            if hit.is_some() {
                state.selected_shape = hit;
                state.selected_combine = None;
            }
        }
    }
}

/// 绘制单个基础形状
fn draw_sandbox_shape(
    painter: &egui::Painter,
    shape: &SandboxShape,
    is_selected: bool,
    w2c: &dyn Fn(f32, f32) -> Pos2,
    scale: f32,
) {
    let [r, g, b, a] = shape.color;
    let fill_alpha = if is_selected { (a as u16 + 50).min(200) as u8 } else { a };
    let fill = Color32::from_rgba_unmultiplied(r, g, b, fill_alpha);
    let stroke_c = if is_selected {
        Color32::from_rgb(255, 255, 100)
    } else {
        Color32::from_rgba_unmultiplied(r, g, b, (a as u16 + 100).min(255) as u8)
    };
    let sw = if is_selected { 2.5 } else { 1.0 };

    match shape.kind {
        ShapeKind::Rect => {
            let tl = w2c(shape.rect_x0 as f32, shape.rect_y0 as f32);
            let br = w2c(shape.rect_x1 as f32, shape.rect_y1 as f32);
            let rect = EguiRect::from_min_max(tl, br);
            painter.rect_filled(rect, 0.0, fill);
            painter.rect_stroke(rect, 0.0, Stroke::new(sw, stroke_c));
        }
        ShapeKind::Ellipse => {
            let center = w2c(shape.ell_cx as f32, shape.ell_cy as f32);
            let radius = Vec2::new(shape.ell_rx as f32 * scale, shape.ell_ry as f32 * scale);
            painter.add(egui::Shape::ellipse_filled(center, radius, fill));
            painter.add(egui::Shape::ellipse_stroke(center, radius, Stroke::new(sw, stroke_c)));
        }
        ShapeKind::Trapezoid => {
            let p0 = w2c(shape.trap_top_x0 as f32, shape.trap_y_top as f32);
            let p1 = w2c(shape.trap_top_x1 as f32, shape.trap_y_top as f32);
            let p2 = w2c(shape.trap_bot_x1 as f32, shape.trap_y_bot as f32);
            let p3 = w2c(shape.trap_bot_x0 as f32, shape.trap_y_bot as f32);
            painter.add(egui::Shape::convex_polygon(
                vec![p0, p1, p2, p3],
                fill,
                Stroke::new(sw, stroke_c),
            ));
        }
        ShapeKind::Column => {
            let top = w2c(shape.col_x as f32, shape.col_y_start as f32);
            let bot = w2c(shape.col_x as f32 + 1.0, shape.col_y_end as f32);
            let rect = EguiRect::from_min_max(top, bot);
            painter.rect_filled(rect, 0.0, fill);
            painter.rect_stroke(rect, 0.0, Stroke::new(sw, stroke_c));
        }
    }

    // 标签
    let bb = shape.bounding_box();
    let label_pos = w2c(
        (bb.x_min + bb.x_max) as f32 / 2.0,
        bb.y_min as f32,
    );
    painter.text(
        Pos2::new(label_pos.x, label_pos.y - 6.0),
        egui::Align2::CENTER_BOTTOM,
        &shape.label,
        egui::FontId::proportional(10.0),
        Color32::from_rgba_unmultiplied(r, g, b, 220),
    );
}

/// 绘制组合运算结果（通过像素采样实现精确预览）
fn draw_combine_result(
    painter: &egui::Painter,
    left: &SandboxShape,
    right: &SandboxShape,
    comb: &CombineNode,
    is_selected: bool,
    w2c: &dyn Fn(f32, f32) -> Pos2,
    scale: f32,
    world_size: (u32, u32),
) {
    let [r, g, b, a] = comb.color;
    let fill_alpha = if is_selected { (a as u16 + 50).min(200) as u8 } else { a };
    let fill = Color32::from_rgba_unmultiplied(r, g, b, fill_alpha);

    // 计算采样范围（两形状 bbox 的并集）
    let bb_l = left.bounding_box();
    let bb_r = right.bounding_box();
    let bb = match comb.op {
        SetOp::Union => bb_l.union(bb_r),
        SetOp::Intersect => bb_l.intersect(bb_r),
        SetOp::Subtract => bb_l, // 差集范围 = A 的 bbox
    };

    if bb.is_empty() {
        return;
    }

    // 限制到世界范围
    let x0 = bb.x_min.max(0);
    let y0 = bb.y_min.max(0);
    let x1 = bb.x_max.min(world_size.0 as i32);
    let y1 = bb.y_max.min(world_size.1 as i32);

    // 计算像素大小（屏幕空间）来决定采样步长
    let pixel_size = scale;
    // 如果每个世界像素在屏幕上小于 2px，跳步采样
    let step = if pixel_size < 1.0 {
        (1.0 / pixel_size).ceil() as i32
    } else {
        1
    }.max(1).min(8);

    // 像素级采样绘制
    let dot_size = (pixel_size * step as f32).max(1.0);

    let mut y = y0;
    while y < y1 {
        let mut x = x0;
        while x < x1 {
            let in_left = left.contains(x, y);
            let in_right = right.contains(x, y);
            let in_result = match comb.op {
                SetOp::Union => in_left || in_right,
                SetOp::Intersect => in_left && in_right,
                SetOp::Subtract => in_left && !in_right,
            };
            if in_result {
                let p = w2c(x as f32, y as f32);
                let r = EguiRect::from_min_size(p, Vec2::splat(dot_size));
                painter.rect_filled(r, 0.0, fill);
            }
            x += step;
        }
        y += step;
    }

    // 组合 bbox 轮廓
    let stroke_c = if is_selected {
        Color32::from_rgb(255, 255, 100)
    } else {
        Color32::from_rgba_unmultiplied(r, g, b, 180)
    };
    let sw = if is_selected { 2.0 } else { 1.0 };
    let tl = w2c(bb.x_min as f32, bb.y_min as f32);
    let br = w2c(bb.x_max as f32, bb.y_max as f32);
    painter.rect_stroke(
        EguiRect::from_min_max(tl, br),
        0.0,
        Stroke::new(sw, stroke_c),
    );
}

// ═══════════════════════════════════════════════════════════
// 详细面板（参数编辑 + 代码）
// ═══════════════════════════════════════════════════════════

fn draw_detail_panel(ui: &mut Ui, state: &mut ShapeSandboxState) {
    // 如果选中了基础形状，编辑其参数
    if let Some(idx) = state.selected_shape {
        if idx < state.shapes.len() {
            draw_shape_editor(ui, &mut state.shapes[idx]);
            return;
        }
    }

    // 如果选中了组合，显示组合信息
    if let Some(ci) = state.selected_combine {
        if ci < state.combines.len() {
            draw_combine_detail(ui, &state.combines[ci], &state.shapes);
            return;
        }
    }

    ui.weak("选择一个形状或组合查看详情");
}

fn draw_shape_editor(ui: &mut Ui, shape: &mut SandboxShape) {
    ui.strong(format!("✏ {} — {}", shape.label, shape.kind.display_name()));
    ui.add_space(4.0);

    // 标签
    ui.horizontal(|ui| {
        ui.label("标签:");
        ui.text_edit_singleline(&mut shape.label);
    });

    ui.add_space(2.0);

    // 参数滑块（根据 kind）
    match shape.kind {
        ShapeKind::Rect => {
            egui::Grid::new("rect_editor").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                ui.label("x0:");
                ui.add(egui::DragValue::new(&mut shape.rect_x0).speed(1));
                ui.end_row();
                ui.label("y0:");
                ui.add(egui::DragValue::new(&mut shape.rect_y0).speed(1));
                ui.end_row();
                ui.label("x1:");
                ui.add(egui::DragValue::new(&mut shape.rect_x1).speed(1));
                ui.end_row();
                ui.label("y1:");
                ui.add(egui::DragValue::new(&mut shape.rect_y1).speed(1));
                ui.end_row();
            });
        }
        ShapeKind::Ellipse => {
            egui::Grid::new("ell_editor").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                ui.label("cx:");
                ui.add(egui::DragValue::new(&mut shape.ell_cx).speed(1.0));
                ui.end_row();
                ui.label("cy:");
                ui.add(egui::DragValue::new(&mut shape.ell_cy).speed(1.0));
                ui.end_row();
                ui.label("rx:");
                ui.add(egui::DragValue::new(&mut shape.ell_rx).speed(1.0).clamp_range(0.0..=f64::MAX));
                ui.end_row();
                ui.label("ry:");
                ui.add(egui::DragValue::new(&mut shape.ell_ry).speed(1.0).clamp_range(0.0..=f64::MAX));
                ui.end_row();
            });
        }
        ShapeKind::Trapezoid => {
            egui::Grid::new("trap_editor").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                ui.label("y_top:");
                ui.add(egui::DragValue::new(&mut shape.trap_y_top).speed(1));
                ui.end_row();
                ui.label("y_bot:");
                ui.add(egui::DragValue::new(&mut shape.trap_y_bot).speed(1));
                ui.end_row();
                ui.label("上边 x0:");
                ui.add(egui::DragValue::new(&mut shape.trap_top_x0).speed(1.0));
                ui.end_row();
                ui.label("上边 x1:");
                ui.add(egui::DragValue::new(&mut shape.trap_top_x1).speed(1.0));
                ui.end_row();
                ui.label("下边 x0:");
                ui.add(egui::DragValue::new(&mut shape.trap_bot_x0).speed(1.0));
                ui.end_row();
                ui.label("下边 x1:");
                ui.add(egui::DragValue::new(&mut shape.trap_bot_x1).speed(1.0));
                ui.end_row();
            });
        }
        ShapeKind::Column => {
            egui::Grid::new("col_editor").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                ui.label("x:");
                ui.add(egui::DragValue::new(&mut shape.col_x).speed(1));
                ui.end_row();
                ui.label("y_start:");
                ui.add(egui::DragValue::new(&mut shape.col_y_start).speed(1));
                ui.end_row();
                ui.label("y_end:");
                ui.add(egui::DragValue::new(&mut shape.col_y_end).speed(1));
                ui.end_row();
            });
        }
    }

    ui.add_space(4.0);

    // 数学描述
    ui.horizontal(|ui| {
        ui.label("数学:");
        ui.monospace(shape.math_desc());
    });

    // Rust 代码
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.label("代码:");
        let code = shape.code_snippet();
        ui.monospace(&code);
        if ui.small_button("📋").on_hover_text("复制代码").clicked() {
            ui.output_mut(|o| o.copied_text = code);
        }
    });
}

fn draw_combine_detail(ui: &mut Ui, comb: &CombineNode, shapes: &[SandboxShape]) {
    let left_label = shapes.get(comb.left).map(|s| s.label.as_str()).unwrap_or("?");
    let right_label = shapes.get(comb.right).map(|s| s.label.as_str()).unwrap_or("?");

    ui.strong(format!(
        "📐 组合: {} {} {}",
        left_label, comb.op.symbol(), right_label
    ));
    ui.add_space(4.0);

    egui::Grid::new("combine_detail").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
        ui.label("运算:");
        ui.monospace(comb.op.label());
        ui.end_row();

        ui.label("左操作数:");
        ui.monospace(format!("[{}] {}", comb.left, left_label));
        ui.end_row();

        ui.label("右操作数:");
        ui.monospace(format!("[{}] {}", comb.right, right_label));
        ui.end_row();
    });

    // 代码
    ui.add_space(4.0);
    if let (Some(left), Some(right)) = (shapes.get(comb.left), shapes.get(comb.right)) {
        let code = format!(
            "{}\n{}\nlet result = a{}(b);",
            left.code_snippet().replace("shape", "a"),
            right.code_snippet().replace("shape", "b"),
            comb.op.code(),
        );
        ui.label("代码:");
        ui.monospace(&code);
        if ui.small_button("📋 复制代码").clicked() {
            ui.output_mut(|o| o.copied_text = code);
        }
    }
}
