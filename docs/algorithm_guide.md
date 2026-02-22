# 算法开发指南

本文档面向希望为 Lwd 编写新的世界生成算法的开发者。内容包括：资产文件结构、引擎 API、几何图形系统和完整开发教程。

---

## 目录

- [算法开发指南](#算法开发指南)
  - [目录](#目录)
  - [概念模型](#概念模型)
  - [资产文件](#资产文件)
    - [blocks.json — 方块定义](#blocksjson--方块定义)
    - [biome.json — 环境定义](#biomejson--环境定义)
    - [world.json — 世界配置](#worldjson--世界配置)
      - [world\_sizes — 世界尺寸规格](#world_sizes--世界尺寸规格)
      - [layers — 层级定义](#layers--层级定义)
  - [算法接口](#算法接口)
    - [PhaseAlgorithm trait](#phasealgorithm-trait)
    - [PhaseMeta 与步骤/参数声明](#phasemeta-与步骤参数声明)
    - [参数类型](#参数类型)
  - [执行上下文（RuntimeContext）](#执行上下文runtimecontext)
    - [字段一览](#字段一览)
    - [层级查询 API](#层级查询-api)
    - [跨步骤共享状态](#跨步骤共享状态)
  - [几何图形系统](#几何图形系统)
    - [Shape trait](#shape-trait)
    - [四种基础形状](#四种基础形状)
      - [Rect — 矩形](#rect--矩形)
      - [Ellipse — 椭圆](#ellipse--椭圆)
      - [Trapezoid — 梯形](#trapezoid--梯形)
      - [Column — 垂直列](#column--垂直列)
    - [集合运算](#集合运算)
    - [填充函数](#填充函数)
    - [形状日志](#形状日志)
  - [世界与环境数据](#世界与环境数据)
    - [World — 方块网格](#world--方块网格)
    - [BiomeMap — 环境网格](#biomemap--环境网格)
  - [开发教程：从零添加一个算法](#开发教程从零添加一个算法)
    - [第一步：创建模块文件](#第一步创建模块文件)
    - [第二步：定义参数](#第二步定义参数)
    - [第三步：实现 PhaseAlgorithm](#第三步实现-phasealgorithm)
    - [第四步：实现步骤逻辑](#第四步实现步骤逻辑)
    - [第五步：注册到管线](#第五步注册到管线)
  - [现有算法参考](#现有算法参考)

---

## 概念模型

Lwd 引擎将世界生成组织为线性管线：

```
Pipeline = [Phase 1] → [Phase 2] → [Phase 3] → ...
                ↓
        [SubStep 0] → [SubStep 1] → ... → [SubStep N]
```

- **Phase**（阶段）：对应一个 `PhaseAlgorithm` 实现，代表一个大的生成主题（如"环境判定"、"地形雕刻"、"矿物分布"）
- **SubStep**（子步骤）：Phase 内的有序操作单元，各步骤可独立执行和回退

引擎保证：
- 每个 SubStep 收到的 RNG 是从 `(主种子, 步骤索引, 世界尺寸)` 确定性派生的 → 相同输入 = 完全相同输出
- 回退通过重置+重放实现 → 步骤不需要自己实现撤销
- UI 中的步骤列表、参数面板、进度条全部从 `meta()` 自动生成 → 算法只需声明，不需要写 UI 代码

---

## 资产文件

资产位于 `src/assets/`，通过 `include_str!` 编译时嵌入。

### blocks.json — 方块定义

定义了 43 种方块。每个方块的结构：

```json
{
  "1": {
    "name": "土块",
    "rgba": [151, 107, 75, 255],
    "description": "棕色泥土",
    "category": "基础"
  }
}
```

| 字段 | 类型 | 说明 |
|------|------|------|
| key（外层） | string | 方块 ID（`"0"` \~ `"43"`） |
| `name` | string | 中文显示名称 |
| `rgba` | `[u8; 4]` | 渲染颜色 `[R, G, B, A]`，A=255 不透明 |
| `description` | string | 外观描述 |
| `category` | string | 分类（基础、植物、冰雪、矿石、地狱、结构、液体、装饰、天空、特殊群系、物品、邪恶、特殊） |

方块 ID `0` 为空气（透明）。在算法中通过 `ctx.blocks` 可按索引查找。

### biome.json — 环境定义

定义了 10 种环境。每个环境的结构：

```json
{
  "1": {
    "key": "ocean",
    "name": "海洋",
    "overlay_color": [30, 100, 200, 80],
    "description": "世界两侧的深蓝区域"
  }
}
```

| 字段 | 类型 | 说明 |
|------|------|------|
| key（外层） | string | 环境 ID（`"1"` \~ `"10"`） |
| `key` | string | 英文标识符，算法中通过此 key 查找 ID |
| `name` | string | 中文显示名称 |
| `overlay_color` | `[u8; 4]` | 覆盖预览颜色，A 通常为 80（半透明） |
| `description` | string | 环境描述 |

特殊值：`BiomeId = 0` 为 `BIOME_UNASSIGNED`（未分配），不在 JSON 中定义。

### world.json — 世界配置

两个顶层节点：

#### world_sizes — 世界尺寸规格

```json
{
  "small":  { "width": 4200, "height": 1200, "description": "小世界" },
  "medium": { "width": 6400, "height": 1800, "description": "中世界" },
  "large":  { "width": 8400, "height": 2400, "description": "大世界" },
  "custom": { "width": null, "height": null, "description": "自定义尺寸" }
}
```

#### layers — 层级定义

```json
{
  "space":       { "start_percent": 0,  "end_percent": 10,  "short_name": "太空", "description": "..." },
  "surface":     { "start_percent": 10, "end_percent": 30,  "short_name": "地表", "description": "..." },
  "underground": { "start_percent": 30, "end_percent": 40,  "short_name": "地下", "description": "..." },
  "cavern":      { "start_percent": 40, "end_percent": 85,  "short_name": "洞穴", "description": "..." },
  "hell":        { "start_percent": 85, "end_percent": 100, "short_name": "地狱", "description": "..." }
}
```

| 字段 | 类型 | 说明 |
|------|------|------|
| key（外层） | string | 层级标识符，用于 `ctx.layer_range("surface")` |
| `start_percent` | u8 | 层级起始位置（世界高度的百分比） |
| `end_percent` | u8 | 层级结束位置 |
| `short_name` | string | 中文短名（可视化覆盖层显示用） |
| `description` | string | 层级描述 |

层级百分比可被用户在 UI 中覆盖。算法应始终通过 `RuntimeContext` 的层级查询 API 获取实际值，不要硬编码百分比。

---

## 算法接口

### PhaseAlgorithm trait

```rust
pub trait PhaseAlgorithm {
    /// 声明阶段元数据（名称、步骤列表、参数定义）
    fn meta(&self) -> PhaseMeta;

    /// 执行第 step_index 个子步骤（0-based）
    fn execute(&mut self, step_index: usize, ctx: &mut RuntimeContext) -> Result<(), String>;

    /// 序列化当前参数为 JSON（用于快照和持久化）
    fn get_params(&self) -> serde_json::Value { serde_json::json!({}) }

    /// 从 JSON 恢复参数
    fn set_params(&mut self, _params: &serde_json::Value) {}

    /// 管线重置时清理内部状态
    fn on_reset(&mut self) {}
}
```

### PhaseMeta 与步骤/参数声明

```rust
pub struct PhaseMeta {
    pub id: String,             // 唯一英文 ID，如 "biome_division"
    pub name: String,           // 显示名称，如 "环境判定"
    pub description: String,    // 阶段描述
    pub steps: Vec<StepMeta>,   // 子步骤列表（有序）
    pub params: Vec<ParamDef>,  // 可调参数定义
}

pub struct StepMeta {
    pub display_index: u32,        // 显示编号（1, 2, 3…）
    pub name: String,              // 步骤名
    pub description: String,       // 步骤描述
    pub doc_url: Option<String>,   // 文档链接（可选，显示在步骤列表中）
}
```

### 参数类型

```rust
pub struct ParamDef {
    pub key: String,                // 参数键名（英文 snake_case）
    pub name: String,               // 中文显示名
    pub description: String,        // 参数说明（UI 中 ℹ 图标悬浮显示）
    pub param_type: ParamType,      // 类型约束
    pub default: serde_json::Value, // 默认值
    pub group: Option<String>,      // UI 分组（None = 不分组）
}

pub enum ParamType {
    Float { min: f64, max: f64 },   // → 浮点数滑块
    Int { min: i64, max: i64 },     // → 整数滑块
    Bool,                           // → 复选框
    Text,                           // → 文本输入框
    Enum { options: Vec<String> },  // → 下拉菜单
}
```

UI 面板从 `ParamDef` 列表自动生成对应控件，按 `group` 分组为可折叠区域。

---

## 执行上下文（RuntimeContext）

### 字段一览

| 字段 | 类型 | 读写 | 说明 |
|------|------|------|------|
| `world` | `&mut World` | 读写 | 方块网格 |
| `profile` | `&WorldProfile` | 只读 | 世界尺寸和层级定义 |
| `blocks` | `&[BlockDefinition]` | 只读 | 全量方块定义表 |
| `biomes` | `&[BiomeDefinition]` | 只读 | 全量环境定义表 |
| `rng` | `&mut StdRng` | 读写 | 本步骤的确定性 RNG |
| `biome_map` | `&mut Option<BiomeMap>` | 读写 | 环境地图（首个步骤需创建） |
| `shared` | `&mut HashMap<String, Box<dyn Any>>` | 读写 | 跨步骤共享数据 |
| `shape_log` | `&mut Vec<ShapeRecord>` | 写 | 几何形状日志（供 UI 预览） |

### 层级查询 API

不要硬编码百分比或像素值——使用这些方法获取用户可能修改过的实际层级范围：

| 方法 | 返回值 | 示例 |
|------|--------|------|
| `ctx.layer_range("surface")` | `Option<(f64, f64)>` | `Some((0.10, 0.30))` |
| `ctx.layer_start("cavern")` | `Option<f64>` | `Some(0.40)` |
| `ctx.layer_end("hell")` | `Option<f64>` | `Some(1.0)` |
| `ctx.layer_range_px("surface")` | `Option<(u32, u32)>` | `Some((120, 360))` |
| `ctx.layer_start_px("space")` | `Option<u32>` | `Some(0)` |
| `ctx.layer_end_px("underground")` | `Option<u32>` | `Some(480)` |

key 即 `world.json` 中 `layers` 节点的键名：`space`、`surface`、`underground`、`cavern`、`hell`。

### 跨步骤共享状态

`ctx.shared` 是一个 `HashMap<String, Box<dyn Any>>`，用于在步骤之间传递中间数据：

```rust
// 写入（某个步骤中）
let heightmap = vec![0u32; w as usize];
ctx.shared.insert("heightmap".into(), Box::new(heightmap));

// 读取（后续步骤中）
let hm = ctx.shared.get("heightmap")
    .and_then(|v| v.downcast_ref::<Vec<u32>>())
    .expect("heightmap 未初始化");
```

---

## 几何图形系统

几何图形系统用于定义和填充环境区域的形状。所有类型定义在 `src/core/geometry.rs`。

### Shape trait

```rust
pub trait Shape: Sync {
    fn contains(&self, x: i32, y: i32) -> bool;  // 点是否在形状内
    fn bounding_box(&self) -> BoundingBox;        // 轴对齐包围盒
    fn type_name(&self) -> &'static str;          // 显示名（如 "矩形"）
}
```

要求 `Sync`，因为填充函数可能使用 rayon 并行。

### 四种基础形状

#### Rect — 矩形

```rust
let r = Rect::new(x0, y0, x1, y1);           // 左上 (x0,y0) 到右下 (x1,y1)，不含右下边界
let r = Rect::from_center(cx, cy, hw, hh);   // 从中心点和半宽/半高构造
```

判定条件：`x ∈ [x0, x1) && y ∈ [y0, y1)`

#### Ellipse — 椭圆

```rust
let e = Ellipse::new(cx, cy, rx, ry);  // 中心 (cx,cy)，半径 (rx,ry)
```

判定条件：$(x-cx)^2/rx^2 + (y-cy)^2/ry^2 \leq 1$

#### Trapezoid — 梯形

```rust
let t = Trapezoid::new(y_top, y_bot, top_x0, top_x1, bot_x0, bot_x1);
let t = Trapezoid::from_center(cx, y_top, y_bot, top_hw, bot_hw);  // 对称梯形
```

判定条件：从上边到下边线性插值 x 范围。

#### Column — 垂直列

```rust
let c = Column::new(x, y_start, y_end);  // 单像素宽的垂直线段
```

判定条件：`x == self.x && y ∈ [y_start, y_end)`

### 集合运算

三种集合组合器，通过 `ShapeCombine` trait 的链式方法调用：

```rust
use crate::core::geometry::*;

// 并集：A ∪ B
let shape = rect.union(ellipse);

// 交集：A ∩ B
let shape = rect.intersect(ellipse);

// 差集：A − B（在 A 中但不在 B 中）
let shape = rect.subtract(ellipse);

// 链式组合
let complex = big_rect
    .subtract(hole_ellipse)
    .union(small_rect);
```

组合后的结果仍然实现 `Shape`，可以继续组合。

### 填充函数

```rust
// 无条件填充：形状区域内全部设为指定 biome
geometry::fill_biome(&shape, biome_map, biome_id);

// 条件填充：仅当 filter(当前值) 返回 true 时才写入
geometry::fill_biome_if(&shape, biome_map, biome_id, |current| current == BIOME_UNASSIGNED);

// 区域检查：形状内是否所有格子都满足条件（step 为采样步长）
let all_empty = geometry::shape_all_match(&shape, biome_map, 1, |id| id == BIOME_UNASSIGNED);
```

所有填充函数会根据面积自动选择串行或并行（rayon）路径，阈值默认 50,000 像素。

### 形状日志

每次使用形状后，应将记录推入 `ctx.shape_log`，供 UI 的几何预览窗口展示：

```rust
ctx.shape_log.push(ShapeRecord {
    label: "左侧海洋".into(),
    bbox: rect.bounding_box(),
    color: algo.biome_color(ocean_id),  // 或自定义 [R,G,B,A]
    params: ShapeParams::from_rect(&rect),
});
```

`ShapeParams` 提供四种构造方法：`from_rect`、`from_ellipse`、`from_trapezoid`、`from_column`。对于组合形状使用 `ShapeParams::Composite { description }`。

---

## 世界与环境数据

### World — 方块网格

```rust
let (w, h) = (ctx.world.width, ctx.world.height);

// 读写单个方块
let block_id = ctx.world.get(x, y);         // Option<u8>
let block_id = ctx.world.get_or_air(x, y);  // 越界返回 0 (空气)
ctx.world.set(x, y, block_id);

// 批量填充
ctx.world.fill_rect(x0, y0, x1, y1, block_id);  // 自动并行
ctx.world.fill_column(x, y_start, y_end, block_id);
```

方块以 `u8` ID 存储，行优先平铺。大面积 `fill_rect` 超过阈值时自动启用 rayon。

### BiomeMap — 环境网格

```rust
// 首个步骤中创建
*ctx.biome_map = Some(BiomeMap::new_filled(w, h, BIOME_UNASSIGNED));

// 后续步骤中使用
let bm = ctx.biome_map.as_mut().expect("BiomeMap 未初始化");

// 读写
let id = bm.get(x, y);
bm.set(x, y, biome_id);

// 通常不直接操作，而是通过 geometry::fill_biome 填充
```

---

## 开发教程：从零添加一个算法

本节以一个假设的"地形雕刻"算法为例，展示完整的开发流程。

### 第一步：创建模块文件

```
src/algorithms/
├── mod.rs                    ← 已有，需在此导出新模块
└── terrain_carving/
    ├── mod.rs                ← 算法主体
    ├── params.rs             ← 参数定义
    ├── step_fill_dirt.rs     ← 子步骤 1
    └── step_carve_caves.rs   ← 子步骤 2
```

### 第二步：定义参数

```rust
// src/algorithms/terrain_carving/params.rs
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TerrainCarvingParams {
    pub dirt_depth: f64,      // 土层深度（世界高度的比例）
    pub cave_density: f64,    // 洞穴密度
    pub cave_min_radius: u32, // 洞穴最小半径
    pub cave_max_radius: u32, // 洞穴最大半径
}

impl Default for TerrainCarvingParams {
    fn default() -> Self {
        Self {
            dirt_depth: 0.05,
            cave_density: 0.3,
            cave_min_radius: 8,
            cave_max_radius: 40,
        }
    }
}
```

### 第三步：实现 PhaseAlgorithm

```rust
// src/algorithms/terrain_carving/mod.rs
mod params;
mod step_fill_dirt;
mod step_carve_caves;

use crate::generation::algorithm::*;
pub use params::TerrainCarvingParams;

pub struct TerrainCarvingAlgorithm {
    pub params: TerrainCarvingParams,
}

impl TerrainCarvingAlgorithm {
    pub fn new() -> Self {
        Self { params: TerrainCarvingParams::default() }
    }
}

impl PhaseAlgorithm for TerrainCarvingAlgorithm {
    fn meta(&self) -> PhaseMeta {
        PhaseMeta {
            id: "terrain_carving".into(),
            name: "地形雕刻".into(),
            description: "在已判定的环境区域内填充方块和雕刻洞穴".into(),
            steps: vec![
                StepMeta {
                    display_index: 1,
                    name: "填充土层".into(),
                    description: "在地表层填充泥土方块".into(),
                    doc_url: None,
                },
                StepMeta {
                    display_index: 2,
                    name: "雕刻洞穴".into(),
                    description: "在洞穴层随机挖掘空腔".into(),
                    doc_url: None,
                },
            ],
            params: vec![
                ParamDef {
                    key: "dirt_depth".into(),
                    name: "土层深度".into(),
                    description: "地表下方泥土层的厚度（比例）".into(),
                    param_type: ParamType::Float { min: 0.01, max: 0.2 },
                    default: serde_json::json!(0.05),
                    group: Some("地表".into()),
                },
                ParamDef {
                    key: "cave_density".into(),
                    name: "洞穴密度".into(),
                    description: "数值越大洞穴越多".into(),
                    param_type: ParamType::Float { min: 0.0, max: 1.0 },
                    default: serde_json::json!(0.3),
                    group: Some("洞穴".into()),
                },
                // ... 更多参数
            ],
        }
    }

    fn execute(&mut self, step_index: usize, ctx: &mut RuntimeContext) -> Result<(), String> {
        match step_index {
            0 => step_fill_dirt::execute(self, ctx),
            1 => step_carve_caves::execute(self, ctx),
            _ => Err(format!("无效步骤索引: {step_index}")),
        }
    }

    fn get_params(&self) -> serde_json::Value {
        serde_json::to_value(&self.params).unwrap_or_default()
    }

    fn set_params(&mut self, params: &serde_json::Value) {
        if let Ok(p) = serde_json::from_value::<TerrainCarvingParams>(params.clone()) {
            self.params = p;
        }
    }
}
```

### 第四步：实现步骤逻辑

```rust
// src/algorithms/terrain_carving/step_fill_dirt.rs
use crate::core::geometry::{self, Rect, ShapeRecord, ShapeParams};
use crate::generation::algorithm::RuntimeContext;
use super::TerrainCarvingAlgorithm;

pub fn execute(algo: &TerrainCarvingAlgorithm, ctx: &mut RuntimeContext) -> Result<(), String> {
    let (w, h) = (ctx.world.width, ctx.world.height);

    // 1. 通过层级 API 获取地表范围（不要硬编码！）
    let (surface_start, surface_end) = ctx.layer_range_px("surface")
        .ok_or("未找到 surface 层级")?;

    // 2. 计算土层像素厚度
    let dirt_rows = (h as f64 * algo.params.dirt_depth) as i32;

    // 3. 创建填充矩形
    let dirt_rect = Rect::new(0, surface_start as i32, w as i32, surface_start as i32 + dirt_rows);

    // 4. 填充方块（假设 block_id=1 为泥土）
    ctx.world.fill_rect(0, surface_start, w, (surface_start + dirt_rows as u32).min(surface_end), 1);

    // 5. 记录形状日志
    ctx.shape_log.push(ShapeRecord {
        label: "地表土层".into(),
        bbox: dirt_rect.bounding_box(),
        color: [151, 107, 75, 120],
        params: ShapeParams::from_rect(&dirt_rect),
    });

    Ok(())
}
```

### 第五步：注册到管线

编辑 `src/generation/mod.rs`：

```rust
use crate::algorithms::terrain_carving::TerrainCarvingAlgorithm;

pub fn build_pipeline(
    seed: u64,
    biome_definitions: Vec<BiomeDefinition>,
    layer_definitions: &[LayerDefinition],
) -> GenerationPipeline {
    let mut pipeline = GenerationPipeline::new(seed, biome_definitions.clone());

    // Phase 1: 环境判定（已有）
    pipeline.register(Box::new(BiomeDivisionAlgorithm::new(&biome_definitions, layer_definitions)));

    // Phase 2: 地形雕刻（新增）
    pipeline.register(Box::new(TerrainCarvingAlgorithm::new()));

    pipeline
}
```

注册完成后，引擎自动：
1. 在步骤列表中显示新阶段和子步骤
2. 生成参数编辑面板
3. 在执行时调用 `execute()`
4. 在几何预览中展示 `shape_log`

---

## 现有算法参考

当前引擎注册了一个 Phase——**环境判定**（`BiomeDivisionAlgorithm`），包含 9 个子步骤和 30+ 可调参数。

| 步骤 | 名称 | 使用形状 | 填充方式 |
|------|------|----------|----------|
| 0 | 太空/地狱填充 | Rect ×2 | `fill_biome` |
| 1 | 海洋生成 | Rect ×2 | `fill_biome` |
| 2 | 森林生成 | Rect | `fill_biome_if`（仅空白区域） |
| 3 | 丛林生成 | Trapezoid | `fill_biome_if` |
| 4 | 雪原生成 | Trapezoid | `fill_biome_if` |
| 5 | 沙漠生成 | Rect + Ellipse | `fill_biome_if` + `fill_biome` |
| 6 | 猩红生成 | Rect ×N（随机数量） | `fill_biome_if` |
| 7 | 森林填充 | — | 扫描式扩散 + 填充剩余 |
| 8 | 地块填充 | — | 全扫描填充未分配区域 |

源码位于 `src/algorithms/biome_division/`，每个步骤一个独立文件。建议阅读 `ocean.rs`（最简单，\~50 行）作为上手参考。
