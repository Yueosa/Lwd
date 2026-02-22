//! # 几何图形 API 系统
//!
//! 提供可组合、可注册的几何图形系统。每种图形实现 [`Shape`] trait，
//! 通过 `contains(x, y) -> bool` 判定点是否在形状内部。
//!
//! ## 设计原则
//!
//! - **形状与填充分离**：`Shape` 只负责几何判定，不关心往哪里写、写什么值。
//! - **可组合**：通过 `Union` / `Intersect` / `Subtract` 组合任意形状。
//! - **可预览**：形状可返回 bounding box，供 UI 预览工具使用。
//! - **条件填充**：`fill` 系列函数接受形状 + 条件闭包，统一处理 BIOME_UNASSIGNED 等逻辑。
//!
//! ## 使用示例
//!
//! ```ignore
//! use crate::core::geometry::*;
//!
//! // 创建椭圆
//! let ell = Ellipse::new(400.0, 300.0, 100.0, 80.0);
//!
//! // 创建矩形
//! let rect = Rect::new(350, 100, 450, 200);
//!
//! // 组合：椭圆减去矩形
//! let shape = ell.subtract(rect);
//!
//! // 条件填充到 BiomeMap
//! fill_biome_map(&shape, bm, biome_id, |bm, x, y| {
//!     bm.get(x, y) == BIOME_UNASSIGNED
//! });
//! ```

use rayon::prelude::*;
use std::sync::atomic::{AtomicI64, Ordering};

use super::biome::{BiomeId, BiomeMap};

/// 全局可配置的并行化像素阈值（由 EngineConfig 在启动时设置）
static PARALLEL_PIXEL_THRESHOLD: AtomicI64 = AtomicI64::new(50_000);

/// 获取当前并行阈值
pub fn parallel_threshold() -> i64 {
    PARALLEL_PIXEL_THRESHOLD.load(Ordering::Relaxed)
}

/// 设置并行阈值（由 EngineConfig 初始化时调用）
pub fn set_parallel_threshold(value: i64) {
    PARALLEL_PIXEL_THRESHOLD.store(value, Ordering::Relaxed);
}

// ═══════════════════════════════════════════════════════════
// 形状记录（用于几何预览窗口）
// ═══════════════════════════════════════════════════════════

/// 形状参数（可序列化，用于重建形状）
#[derive(Debug, Clone)]
pub enum ShapeParams {
    Rect { x0: i32, y0: i32, x1: i32, y1: i32 },
    Ellipse { cx: f64, cy: f64, rx: f64, ry: f64 },
    Trapezoid { y_top: i32, y_bot: i32, top_x0: f64, top_x1: f64, bot_x0: f64, bot_x1: f64 },
    Column { x: i32, y_start: i32, y_end: i32 },
    /// 组合形状（交集/并集/差集），仅保存 bbox + 类型名，不可重建
    Composite { description: String },
}

impl ShapeParams {
    /// 从 Rect 构造
    pub fn from_rect(r: &Rect) -> Self {
        ShapeParams::Rect { x0: r.x0, y0: r.y0, x1: r.x1, y1: r.y1 }
    }
    /// 从 Ellipse 构造
    pub fn from_ellipse(e: &Ellipse) -> Self {
        ShapeParams::Ellipse { cx: e.cx, cy: e.cy, rx: e.rx, ry: e.ry }
    }
    /// 从 Trapezoid 构造
    pub fn from_trapezoid(t: &Trapezoid) -> Self {
        ShapeParams::Trapezoid {
            y_top: t.y_top, y_bot: t.y_bot,
            top_x0: t.top_x0, top_x1: t.top_x1,
            bot_x0: t.bot_x0, bot_x1: t.bot_x1,
        }
    }
    /// 从 Column 构造
    pub fn from_column(c: &Column) -> Self {
        ShapeParams::Column { x: c.x, y_start: c.y_start, y_end: c.y_end }
    }

    /// 形状类型标签
    pub fn kind_label(&self) -> &'static str {
        match self {
            ShapeParams::Rect { .. } => "矩形",
            ShapeParams::Ellipse { .. } => "椭圆",
            ShapeParams::Trapezoid { .. } => "梯形",
            ShapeParams::Column { .. } => "列",
            ShapeParams::Composite { .. } => "组合",
        }
    }

    /// 数学描述字符串
    pub fn math_description(&self) -> String {
        match self {
            ShapeParams::Rect { x0, y0, x1, y1 } =>
                format!("x∈[{x0},{x1}), y∈[{y0},{y1})"),
            ShapeParams::Ellipse { cx, cy, rx, ry } =>
                format!("(x-{cx:.0})²/{rx:.0}² + (y-{cy:.0})²/{ry:.0}² ≤ 1"),
            ShapeParams::Trapezoid { y_top, y_bot, top_x0, top_x1, bot_x0, bot_x1 } =>
                format!("y∈[{y_top},{y_bot}), 上[{top_x0:.0},{top_x1:.0}), 下[{bot_x0:.0},{bot_x1:.0})"),
            ShapeParams::Column { x, y_start, y_end } =>
                format!("x={x}, y∈[{y_start},{y_end})"),
            ShapeParams::Composite { description } =>
                description.clone(),
        }
    }
}

/// 一条形状使用记录（每次 fill 操作产生一条）
#[derive(Debug, Clone)]
pub struct ShapeRecord {
    /// 人类可读标签（如 "太空层", "左侧海洋", "真沙漠椭圆"）
    pub label: String,
    /// 包围盒（用于快速绘制轮廓）
    pub bbox: BoundingBox,
    /// 预览颜色 [r, g, b, a]
    pub color: [u8; 4],
    /// 形状参数（可用于重建形状或显示数学公式）
    pub params: ShapeParams,
}

// ═══════════════════════════════════════════════════════════
// 核心 Trait
// ═══════════════════════════════════════════════════════════

/// 轴对齐包围盒（像素坐标，整数）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundingBox {
    pub x_min: i32,
    pub y_min: i32,
    pub x_max: i32,
    pub y_max: i32,
}

impl BoundingBox {
    pub fn new(x_min: i32, y_min: i32, x_max: i32, y_max: i32) -> Self {
        Self { x_min, y_min, x_max, y_max }
    }

    /// 两个 AABB 的并集包围盒
    pub fn union(self, other: Self) -> Self {
        Self {
            x_min: self.x_min.min(other.x_min),
            y_min: self.y_min.min(other.y_min),
            x_max: self.x_max.max(other.x_max),
            y_max: self.y_max.max(other.y_max),
        }
    }

    /// 两个 AABB 的交集包围盒
    pub fn intersect(self, other: Self) -> Self {
        Self {
            x_min: self.x_min.max(other.x_min),
            y_min: self.y_min.max(other.y_min),
            x_max: self.x_max.min(other.x_max),
            y_max: self.y_max.min(other.y_max),
        }
    }

    /// 是否为空（无面积）
    pub fn is_empty(self) -> bool {
        self.x_min >= self.x_max || self.y_min >= self.y_max
    }
}

/// 几何形状 trait
///
/// 每种形状实现此 trait，提供：
/// - `contains(x, y)` 判定点是否在形状内部
/// - `bounding_box()` 返回轴对齐包围盒（用于遍历优化和 UI 预览）
///
/// 要求 `Sync` 以支持 rayon 并行填充。
pub trait Shape: Sync {
    /// 判定 (x, y) 是否在形状内部
    fn contains(&self, x: i32, y: i32) -> bool;

    /// 返回轴对齐包围盒
    fn bounding_box(&self) -> BoundingBox;

    /// 返回形状类型名称（用于 UI 显示）
    fn type_name(&self) -> &'static str;
}

// ═══════════════════════════════════════════════════════════
// 基础形状
// ═══════════════════════════════════════════════════════════

/// 矩形（轴对齐）
#[derive(Debug, Clone)]
pub struct Rect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl Rect {
    pub fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> Self {
        Self {
            x0: x0.min(x1),
            y0: y0.min(y1),
            x1: x0.max(x1),
            y1: y0.max(y1),
        }
    }

    /// 从中心 + 半宽半高创建
    pub fn from_center(cx: i32, cy: i32, half_w: i32, half_h: i32) -> Self {
        Self::new(cx - half_w, cy - half_h, cx + half_w, cy + half_h)
    }
}

impl Shape for Rect {
    fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x0 && x < self.x1 && y >= self.y0 && y < self.y1
    }

    fn bounding_box(&self) -> BoundingBox {
        BoundingBox::new(self.x0, self.y0, self.x1, self.y1)
    }

    fn type_name(&self) -> &'static str { "矩形" }
}

/// 椭圆
#[derive(Debug, Clone)]
pub struct Ellipse {
    pub cx: f64,
    pub cy: f64,
    pub rx: f64,
    pub ry: f64,
}

impl Ellipse {
    pub fn new(cx: f64, cy: f64, rx: f64, ry: f64) -> Self {
        Self { cx, cy, rx: rx.abs(), ry: ry.abs() }
    }
}

impl Shape for Ellipse {
    fn contains(&self, x: i32, y: i32) -> bool {
        if self.rx <= 0.0 || self.ry <= 0.0 {
            return false;
        }
        let dx = (x as f64 - self.cx) / self.rx;
        let dy = (y as f64 - self.cy) / self.ry;
        dx * dx + dy * dy <= 1.0
    }

    fn bounding_box(&self) -> BoundingBox {
        BoundingBox::new(
            (self.cx - self.rx).floor() as i32,
            (self.cy - self.ry).floor() as i32,
            (self.cx + self.rx).ceil() as i32,
            (self.cy + self.ry).ceil() as i32,
        )
    }

    fn type_name(&self) -> &'static str { "椭圆" }
}

/// 梯形（左右边界随 y 线性变化）
#[derive(Debug, Clone)]
pub struct Trapezoid {
    pub y_top: i32,
    pub y_bot: i32,
    pub top_x0: f64,
    pub top_x1: f64,
    pub bot_x0: f64,
    pub bot_x1: f64,
}

impl Trapezoid {
    pub fn new(y_top: i32, y_bot: i32, top_x0: f64, top_x1: f64, bot_x0: f64, bot_x1: f64) -> Self {
        Self { y_top, y_bot, top_x0, top_x1, bot_x0, bot_x1 }
    }

    /// 从中心 + 上下半宽创建
    pub fn from_center(
        cx: f64, y_top: i32, y_bot: i32,
        top_half_w: f64, bot_half_w: f64,
    ) -> Self {
        Self {
            y_top, y_bot,
            top_x0: cx - top_half_w,
            top_x1: cx + top_half_w,
            bot_x0: cx - bot_half_w,
            bot_x1: cx + bot_half_w,
        }
    }
}

impl Shape for Trapezoid {
    fn contains(&self, x: i32, y: i32) -> bool {
        if y < self.y_top || y >= self.y_bot || self.y_top >= self.y_bot {
            return false;
        }
        let h = (self.y_bot - self.y_top) as f64;
        let t = (y - self.y_top) as f64 / h;
        let left = self.top_x0 + (self.bot_x0 - self.top_x0) * t;
        let right = self.top_x1 + (self.bot_x1 - self.top_x1) * t;
        (x as f64) >= left && (x as f64) < right
    }

    fn bounding_box(&self) -> BoundingBox {
        let x_min = self.top_x0.min(self.bot_x0).floor() as i32;
        let x_max = self.top_x1.max(self.bot_x1).ceil() as i32;
        BoundingBox::new(x_min, self.y_top, x_max, self.y_bot)
    }

    fn type_name(&self) -> &'static str { "梯形" }
}

/// 垂直列
#[derive(Debug, Clone)]
pub struct Column {
    pub x: i32,
    pub y_start: i32,
    pub y_end: i32,
}

impl Column {
    pub fn new(x: i32, y_start: i32, y_end: i32) -> Self {
        Self { x, y_start, y_end }
    }
}

impl Shape for Column {
    fn contains(&self, x: i32, y: i32) -> bool {
        x == self.x && y >= self.y_start && y < self.y_end
    }

    fn bounding_box(&self) -> BoundingBox {
        BoundingBox::new(self.x, self.y_start, self.x + 1, self.y_end)
    }

    fn type_name(&self) -> &'static str { "列" }
}

// ═══════════════════════════════════════════════════════════
// 组合形状
// ═══════════════════════════════════════════════════════════

/// 并集：A ∪ B
pub struct Union<A: Shape, B: Shape> {
    pub a: A,
    pub b: B,
}

impl<A: Shape, B: Shape> Shape for Union<A, B> {
    fn contains(&self, x: i32, y: i32) -> bool {
        self.a.contains(x, y) || self.b.contains(x, y)
    }

    fn bounding_box(&self) -> BoundingBox {
        self.a.bounding_box().union(self.b.bounding_box())
    }

    fn type_name(&self) -> &'static str { "并集" }
}

/// 交集：A ∩ B
pub struct Intersect<A: Shape, B: Shape> {
    pub a: A,
    pub b: B,
}

impl<A: Shape, B: Shape> Shape for Intersect<A, B> {
    fn contains(&self, x: i32, y: i32) -> bool {
        self.a.contains(x, y) && self.b.contains(x, y)
    }

    fn bounding_box(&self) -> BoundingBox {
        self.a.bounding_box().intersect(self.b.bounding_box())
    }

    fn type_name(&self) -> &'static str { "交集" }
}

/// 差集：A - B（在 A 内但不在 B 内）
pub struct Subtract<A: Shape, B: Shape> {
    pub a: A,
    pub b: B,
}

impl<A: Shape, B: Shape> Shape for Subtract<A, B> {
    fn contains(&self, x: i32, y: i32) -> bool {
        self.a.contains(x, y) && !self.b.contains(x, y)
    }

    fn bounding_box(&self) -> BoundingBox {
        self.a.bounding_box()
    }

    fn type_name(&self) -> &'static str { "差集" }
}

// ═══════════════════════════════════════════════════════════
// 组合便捷方法（泛型扩展）
// ═══════════════════════════════════════════════════════════

/// 为所有 Shape 提供组合便捷方法
pub trait ShapeCombine: Shape + Sized {
    /// 与另一个形状取并集
    fn union<B: Shape>(self, other: B) -> Union<Self, B> {
        Union { a: self, b: other }
    }

    /// 与另一个形状取交集
    fn intersect<B: Shape>(self, other: B) -> Intersect<Self, B> {
        Intersect { a: self, b: other }
    }

    /// 减去另一个形状
    fn subtract<B: Shape>(self, other: B) -> Subtract<Self, B> {
        Subtract { a: self, b: other }
    }
}

// 为所有实现 Shape 的类型自动实现 ShapeCombine
impl<T: Shape + Sized> ShapeCombine for T {}

// ═══════════════════════════════════════════════════════════
// 填充函数 —— 将形状应用到 BiomeMap
// ═══════════════════════════════════════════════════════════

/// 将形状填充到 BiomeMap（无条件覆写）
///
/// 自动根据区域大小切换串行/并行路径。
pub fn fill_biome(shape: &dyn Shape, bm: &mut BiomeMap, biome: BiomeId) {
    let bb = shape.bounding_box();
    let x0 = bb.x_min.max(0);
    let y0 = bb.y_min.max(0);
    let x1 = bb.x_max.min(bm.width as i32);
    let y1 = bb.y_max.min(bm.height as i32);

    let area = (x1 - x0) as i64 * (y1 - y0) as i64;
    if area >= parallel_threshold() {
        fill_biome_parallel(shape, bm, biome, x0, y0, x1, y1);
    } else {
        fill_biome_serial(shape, bm, biome, x0, y0, x1, y1);
    }
}

fn fill_biome_serial(
    shape: &dyn Shape, bm: &mut BiomeMap, biome: BiomeId,
    x0: i32, y0: i32, x1: i32, y1: i32,
) {
    for y in y0..y1 {
        for x in x0..x1 {
            if shape.contains(x, y) {
                bm.set(x as u32, y as u32, biome);
            }
        }
    }
}

fn fill_biome_parallel(
    shape: &dyn Shape, bm: &mut BiomeMap, biome: BiomeId,
    x0: i32, y0: i32, x1: i32, y1: i32,
) {
    let w = bm.width as usize;
    let data = bm.data_mut();
    // 按行并行：每行的写入互不竞争
    let rows: Vec<usize> = (y0 as usize..y1 as usize).collect();
    let row_slices = data.chunks_mut(w).enumerate()
        .filter(|(y, _)| *y >= y0 as usize && *y < y1 as usize)
        .map(|(_y, row)| row)
        .collect::<Vec<_>>();

    row_slices.into_par_iter().enumerate().for_each(|(ri, row)| {
        let y = rows[ri] as i32;
        for x in x0..x1 {
            if shape.contains(x, y) {
                row[x as usize] = biome;
            }
        }
    });
}

/// 将形状条件填充到 BiomeMap
///
/// `filter` 闭包接收 (当前格子的 BiomeId)，返回 true 才填充。
/// 自动根据区域大小切换串行/并行路径。
///
/// # 示例
/// ```ignore
/// // 只填充 UNASSIGNED 的格子
/// fill_biome_if(&shape, bm, forest_id, |current| current == BIOME_UNASSIGNED);
///
/// // 填充 UNASSIGNED 或 desert 的格子
/// fill_biome_if(&shape, bm, true_desert_id, |c| c == BIOME_UNASSIGNED || c == desert_id);
/// ```
pub fn fill_biome_if(
    shape: &dyn Shape,
    bm: &mut BiomeMap,
    biome: BiomeId,
    filter: impl Fn(BiomeId) -> bool + Sync,
) {
    let bb = shape.bounding_box();
    let x0 = bb.x_min.max(0);
    let y0 = bb.y_min.max(0);
    let x1 = bb.x_max.min(bm.width as i32);
    let y1 = bb.y_max.min(bm.height as i32);

    let area = (x1 - x0) as i64 * (y1 - y0) as i64;
    if area >= parallel_threshold() {
        fill_biome_if_parallel(shape, bm, biome, &filter, x0, y0, x1, y1);
    } else {
        fill_biome_if_serial(shape, bm, biome, &filter, x0, y0, x1, y1);
    }
}

fn fill_biome_if_serial(
    shape: &dyn Shape, bm: &mut BiomeMap, biome: BiomeId,
    filter: &(impl Fn(BiomeId) -> bool + Sync), x0: i32, y0: i32, x1: i32, y1: i32,
) {
    for y in y0..y1 {
        for x in x0..x1 {
            if shape.contains(x, y) {
                let current = bm.get(x as u32, y as u32);
                if filter(current) {
                    bm.set(x as u32, y as u32, biome);
                }
            }
        }
    }
}

fn fill_biome_if_parallel(
    shape: &dyn Shape, bm: &mut BiomeMap, biome: BiomeId,
    filter: &(impl Fn(BiomeId) -> bool + Sync), x0: i32, y0: i32, x1: i32, y1: i32,
) {
    let w = bm.width as usize;
    let data = bm.data_mut();
    let rows: Vec<usize> = (y0 as usize..y1 as usize).collect();
    let row_slices = data.chunks_mut(w).enumerate()
        .filter(|(y, _)| *y >= y0 as usize && *y < y1 as usize)
        .map(|(_y, row)| row)
        .collect::<Vec<_>>();

    row_slices.into_par_iter().enumerate().for_each(|(ri, row)| {
        let y = rows[ri] as i32;
        for x in x0..x1 {
            if shape.contains(x, y) {
                let current = row[x as usize];
                if filter(current) {
                    row[x as usize] = biome;
                }
            }
        }
    });
}

/// 检查形状区域内是否全部满足条件（用于放置前的空白验证）
///
/// `step` 为采样步长（> 1 可加速大区域检查）
/// 自动根据区域大小切换串行/并行路径，并行模式支持提前退出。
pub fn shape_all_match(
    shape: &dyn Shape,
    bm: &BiomeMap,
    step: i32,
    predicate: impl Fn(BiomeId) -> bool + Sync,
) -> bool {
    let bb = shape.bounding_box();
    let x0 = bb.x_min.max(0);
    let y0 = bb.y_min.max(0);
    let x1 = bb.x_max.min(bm.width as i32);
    let y1 = bb.y_max.min(bm.height as i32);
    let step = step.max(1);

    let area = ((x1 - x0) as i64 / step as i64) * ((y1 - y0) as i64 / step as i64);
    let data = bm.data();
    let w = bm.width as usize;

    if area >= parallel_threshold() {
        // 并行按行检查，支持提前退出
        let ys: Vec<i32> = (0..).map(|i| y0 + i * step).take_while(|&y| y < y1).collect();
        ys.par_iter().all(|&y| {
            let row_start = y as usize * w;
            let mut x = x0;
            while x < x1 {
                if shape.contains(x, y) && !predicate(data[row_start + x as usize]) {
                    return false;
                }
                x += step;
            }
            true
        })
    } else {
        let mut y = y0;
        while y < y1 {
            let mut x = x0;
            while x < x1 {
                if shape.contains(x, y) && !predicate(bm.get(x as u32, y as u32)) {
                    return false;
                }
                x += step;
            }
            y += step;
        }
        true
    }
}

// ═══════════════════════════════════════════════════════════
// 形状注册表（运行时动态注册，供 UI 和预览使用）
// ═══════════════════════════════════════════════════════════

/// 形状类型标识
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShapeKind {
    Rect,
    Ellipse,
    Trapezoid,
    Column,
}

impl ShapeKind {
    /// 所有内置形状类型
    pub fn all() -> &'static [ShapeKind] {
        &[
            ShapeKind::Rect,
            ShapeKind::Ellipse,
            ShapeKind::Trapezoid,
            ShapeKind::Column,
        ]
    }

    /// 显示名称
    pub fn display_name(self) -> &'static str {
        match self {
            ShapeKind::Rect => "矩形",
            ShapeKind::Ellipse => "椭圆",
            ShapeKind::Trapezoid => "梯形",
            ShapeKind::Column => "列",
        }
    }

    /// 数学描述
    pub fn math_description(self) -> &'static str {
        match self {
            ShapeKind::Rect => "轴对齐矩形：x∈[x0,x1), y∈[y0,y1)",
            ShapeKind::Ellipse => "标准椭圆方程：(x-cx)²/rx² + (y-cy)²/ry² ≤ 1",
            ShapeKind::Trapezoid => "左右边界线性插值：t=(y-y_top)/h, x∈[lerp(top,bot,t))",
            ShapeKind::Column => "单像素宽垂直线段：x=固定, y∈[y_start,y_end)",
        }
    }
}
