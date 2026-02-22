# 模块总览

本文档介绍 Lwd 引擎各模块的职责、在软件中的角色以及大致实现思路。

---

## 目录

- [Core — 数据模型层](#core--数据模型层)
- [Config — 配置加载层](#config--配置加载层)
- [Generation — 生成引擎](#generation--生成引擎)
- [Rendering — 渲染层](#rendering--渲染层)
- [Storage — 持久化层](#storage--持久化层)
- [UI — 用户界面层](#ui--用户界面层)
- [Algorithms — 生成算法](#algorithms--生成算法)

---

## Core — 数据模型层

> 源码：[src/core/](../src/core/)

Core 定义了整个引擎的基础数据类型，不依赖 UI 或外部库（除 rayon 并行）。

### World（世界）

世界是一张由方块（tile）构成的二维网格。每个格子存储一个 `u8` 方块 ID（0\~255），以行优先的 `Vec<u8>` 平铺存储。

引擎提供 `get / set / fill_rect / fill_column` 等操作接口。当填充面积超过阈值时，自动切换到 rayon 多线程并行填充。

`WorldProfile` 将尺寸规格（宽×高+描述）和层级定义打包在一起，由 `world.json` 配置文件驱动。

→ [src/core/world.rs](../src/core/world.rs)

### Block（方块）

方块定义包含 ID、名称、颜色和分类。全量方块表从 `blocks.json` 构建，驱动画布的颜色查找表和鼠标悬停信息。当前定义了约 40 种方块，涵盖空气、泥土、石头、沙子、矿石、植被、地狱等分类。

→ [src/core/block.rs](../src/core/block.rs)

### Biome（环境）

环境系统由两部分组成：

- **BiomeDefinition**：从 `biome.json` 加载的环境元数据（ID、key、名称、覆盖色、描述），共 10 种环境
- **BiomeMap**：与世界同尺寸的 2D 网格，记录每个格子的环境 ID。算法通过几何填充函数向 BiomeMap 写入数据

`BiomeContext` 可查询任意坐标的 (环境 + 层级) 组合信息，用于环境标签的显示（如"森林·地表"）。

→ [src/core/biome.rs](../src/core/biome.rs)

### Layer（层级）

层级将世界在垂直方向分为若干区间（如太空 0\~10%、地表 10\~30%、地下 30\~40%、洞穴 40\~85%、地狱 85\~100%）。每个层级定义包含 key、百分比范围、中文短名称和描述，全部从 `world.json` 读取。

`bounds_for_height(h)` 将百分比映射到具体像素行范围，供算法和可视化使用。

→ [src/core/layer.rs](../src/core/layer.rs)

### Geometry（几何图形系统）

引擎提供一套可组合的几何图形 API，用于定义和填充环境区域的形状。

**4 种基础形状：**

| 形状 | 说明 |
|------|------|
| Rect | 轴对齐矩形 |
| Ellipse | 标准椭圆 |
| Trapezoid | 梯形（上下边宽度可不同） |
| Column | 单像素宽的垂直线段 |

**3 种集合运算：** Union（并集）、Intersect（交集）、Subtract（差集），可链式组合任意形状。

所有形状实现 `Shape` trait（`contains(x,y)` + `bounding_box()`），通过 `fill_biome` / `fill_biome_if` 函数批量写入 BiomeMap。填充函数根据面积自动选择串行或并行路径。

每次填充操作会产生 `ShapeRecord` 日志，供几何预览窗口展示。

→ [src/core/geometry.rs](../src/core/geometry.rs)

---

## Config — 配置加载层

> 源码：[src/config/](../src/config/)

Config 负责将 `src/assets/` 下的 JSON 文件反序列化为 Rust 结构体。三个 JSON 分别加载为：

| 文件 | 加载函数 | 产出类型 |
|------|----------|----------|
| `blocks.json` | `load_blocks_config()` | `BTreeMap<u8, BlockConfig>` |
| `biome.json` | `load_biomes_config()` | `BTreeMap<u8, BiomeConfig>` |
| `world.json` | `load_world_config()` | `WorldConfig`（世界尺寸表 + 层级配置表） |

JSON 文件通过 `include_str!` 在编译时嵌入二进制，运行时无外部文件依赖。Config 只做反序列化，不包含业务逻辑——实际的领域模型构建在 Core 层完成。

→ [src/config/blocks.rs](../src/config/blocks.rs)　[src/config/biome.rs](../src/config/biome.rs)　[src/config/world.rs](../src/config/world.rs)

---

## Generation — 生成引擎

> 源码：[src/generation/](../src/generation/)

### PhaseAlgorithm（算法接口）

引擎通过 `PhaseAlgorithm` trait 与算法模块交互。每个算法必须实现：

- `meta()` → 返回自身的元数据（ID、名称、子步骤列表、可调参数定义）
- `execute(step_index, ctx)` → 执行指定子步骤
- `get_params()` / `set_params()` → 参数序列化/反序列化

引擎不硬编码任何算法的具体内容——UI 面板、步骤列表、参数编辑控件全部从 `meta()` 自动生成。

`RuntimeContext` 是算法执行时获取的上下文，包含世界数据、环境地图、方块表、RNG、跨步骤共享状态，以及层级查询 API（`layer_range` / `layer_start_px` 等 6 个接口）。

→ [src/generation/algorithm.rs](../src/generation/algorithm.rs)

### Pipeline（生成管线）

管线管理一组算法模块的注册和有序执行。核心能力：

- **子步骤粒度前进/后退**：每个算法的每个 SubStep 都可以独立执行或回退
- **确定性种子**：每步的 RNG 从 (主种子 + 步骤索引 + 世界尺寸) 确定性派生，保证相同输入 = 相同输出
- **回退策略**：清空世界后从第 0 步重放到目标位置（简单可靠，代价是后期回退较慢）
- **增量执行**：`running_to_end` 模式下由 `AdaptiveBatchSize` 控制每帧执行多少步，通过 EMA 平滑反馈维持 8\~16ms 帧预算

→ [src/generation/pipeline.rs](../src/generation/pipeline.rs)

### Optimizer（性能优化器）

三个运行时优化组件：

| 组件 | 功能 |
|------|------|
| `AdaptiveBatchSize` | EMA 反馈控制每帧批量步数，维持目标帧时间 |
| `TextureUpdateThrottle` | 根据世界像素总量分三档节流纹理刷新频率 |
| `PerfProfiler` | 按步骤记录执行耗时（min/max/avg），生成报告 |

→ [src/generation/optimizer.rs](../src/generation/optimizer.rs)

### Snapshot（快照系统）

`.lwd` 快照是一个 JSON 文件，保存复现一个世界所需的最小信息：种子、世界尺寸 key、层级覆盖值、每个算法的参数。**不保存方块数据**——导入时从头重放即可还原。

同时提供 `export_png` 功能，将世界 1:1 导出为 RGBA PNG 图片。

→ [src/generation/snapshot.rs](../src/generation/snapshot.rs)

---

## Rendering — 渲染层

> 源码：[src/rendering/](../src/rendering/)

渲染分为 CPU 和 GPU 两层协作。

### CPU Canvas

将世界方块数据通过 256-entry 颜色查找表（LUT）转换为像素。支持四种输出模式：全图、降采样、子区域、子区域+LOD。所有转换均使用 rayon 按行并行。

CPU Canvas 的输出用于 minimap 缩略图和 GL Canvas 的输入数据源。

→ [src/rendering/canvas.rs](../src/rendering/canvas.rs)

### GPU Canvas（GL Canvas）

核心渲染器，使用 `glow`（OpenGL 3.1+）在 egui 的 `PaintCallback` 中完成所有画面绘制：

- **棋盘格背景** → GLSL fragment shader 实现，零 CPU 开销
- **世界纹理** → 从 CPU Canvas 获取像素上传为 GL 纹理
- **环境覆盖** → 独立半透明纹理叠加

采用 3× 视口缓冲 + LOD 网格对齐策略，拖拽和缩放时仅在缓冲区耗尽时才重新计算子区域。

→ [src/rendering/gl_canvas.rs](../src/rendering/gl_canvas.rs)

### Viewport（视口状态）

管理缩放比例（0.1×\~20×，默认 0.3×）和偏移量。鼠标滚轮以光标为锚点缩放，拖拽平移。

→ [src/rendering/viewport.rs](../src/rendering/viewport.rs)

---

## Storage — 持久化层

> 源码：[src/storage/](../src/storage/)

所有运行时数据存储在 `~/.local/share/lwd/`（遵循 XDG 规范）。

### 路径管理

三级回退：`$XDG_DATA_HOME/lwd` → `~/.local/share/lwd/` → 可执行文件旁边。启动时自动创建目录结构，并迁移旧版 `generation.runtime.json` 文件。

→ [src/storage/paths.rs](../src/storage/paths.rs)

### runtime.json

统一配置文件，所有模块通过 `merge_field(key, value)` 接口独立读写各自字段，互不干扰。包含三个顶层 key：

| Key | 内容 |
|-----|------|
| `engine` | 引擎调优参数（并行阈值、batch 控制、纹理节流、日志保留等） |
| `layers` | 用户自定义的层级百分比 |
| `ui` | UI 状态（世界尺寸选择、覆盖层开关） |

→ [src/storage/runtime.rs](../src/storage/runtime.rs)　[src/storage/engine_config.rs](../src/storage/engine_config.rs)

### 性能日志

每次世界生成完成后写入独立的 JSON 文件（`logs/perf_YYYYMMDD_HHMMSS.json`），记录种子、世界尺寸、总耗时、每步耗时明细。超过上限（默认 100 条）时自动清理最旧的。

→ [src/storage/perf_log.rs](../src/storage/perf_log.rs)

---

## UI — 用户界面层

> 源码：[src/ui/](../src/ui/)

UI 层使用 egui/eframe 构建，所有面板和窗口的详细操作说明请参阅 **[UI 使用手册](ui_guide.md)**。

### App 主循环

`LianWorldApp` 实现 `eframe::App`，是整个应用的胶合层。每帧处理流程：action 分发 → 增量执行 → 纹理刷新 → 面板渲染。

→ [src/ui/app.rs](../src/ui/app.rs)

### 画布视图

整合 GL 渲染、环境覆盖、层级线、minimap 和鼠标交互。环境标签使用自适应步长扫描 + 碰撞检测，minimap 带视口矩形指示器。

→ [src/ui/canvas_view.rs](../src/ui/canvas_view.rs)

### 控制面板

左侧面板，包含世界尺寸选择（3 挡位 + 自定义输入）、种子输入、渐变进度条、步进控制（±子步骤 / ±阶段）、步骤列表（三色符号标识）、一键生成 / 执行到底、算法参数入口、导出导入按钮等。

→ [src/ui/control_panel.rs](../src/ui/control_panel.rs)

### 子窗口

| 窗口 | 功能 | 源码 |
|------|------|------|
| 层级配置 | 百分比/行数双模式编辑层级范围，智能对齐 | [layer_config.rs](../src/ui/layer_config.rs) |
| 可视化配置 | 4 项独立开关（环境色/环境标签/层级线/层级标签） | [overlay_config.rs](../src/ui/overlay_config.rs) |
| 算法参数 | 从算法元数据自动生成控件（Float / Int / Bool / Text / Enum），分组折叠 | [algo_config.rs](../src/ui/algo_config.rs) |
| 几何预览 | mini-canvas 展示步骤形状 + 形状列表 + 参数详情 | [geo_preview.rs](../src/ui/geo_preview.rs) |
| 图形沙箱 | 多实例交互创建/组合形状，实时预览 + 代码生成 | [shape_sandbox.rs](../src/ui/shape_sandbox.rs) |
| 性能面板 | 查看/编辑引擎调优参数 + 耗时报告 + @历史日志 | [perf_panel.rs](../src/ui/perf_panel.rs) |

### 其他

| 模块 | 功能 | 源码 |
|------|------|------|
| Theme | 粉蓝白主题（暗色基底 + 双强调色 + 步骤符号定义） | [theme.rs](../src/ui/theme.rs) |
| Splash | 启动画面（ASCII 字符画 + 渐变动画 + 闪烁提示） | [splash.rs](../src/ui/splash.rs) |
| Status Bar | 底部状态栏（状态/hover/步骤/尺寸/Seed/FPS/内存） | [status_bar.rs](../src/ui/status_bar.rs) |

---

## Algorithms — 生成算法

> 源码：[src/algorithms/](../src/algorithms/)

算法模块是用户扩展区。当前注册了一个 Phase：**环境判定**（`BiomeDivisionAlgorithm`），包含 9 个子步骤和 30+ 可调参数。

引擎通过 `PhaseAlgorithm` trait 与算法解耦——添加新算法只需实现 trait 并在 `build_pipeline` 中注册。算法开发的完整教程请参阅 **[算法开发指南](algorithm_guide.md)**。

### 当前算法：环境判定（Phase 1）

| 步骤 | 名称 | 说明 |
|------|------|------|
| 1 | 太空/地狱填充 | 初始化 BiomeMap，填充太空层和地狱层 |
| 2 | 海洋生成 | 世界两侧生成海洋矩形 |
| 3 | 森林生成 | 世界中心生成出生点森林 |
| 4 | 丛林生成 | 一侧生成梯形丛林 |
| 5 | 雪原生成 | 另一侧生成梯形雪原 |
| 6 | 沙漠生成 | 空白区域随机放置沙漠矩形 + 深层真沙漠椭圆 |
| 7 | 猩红生成 | 空白区域随机放置猩红矩形 |
| 8 | 森林填充 | 沙漠/猩红边缘扩散 + 剩余空白填森林 |
| 9 | 地块填充 | 未分配区域全部填充地块（岩石） |
