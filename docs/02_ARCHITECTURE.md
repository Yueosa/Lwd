# 架构全景

## 模块关系图

```
  main.rs
  eframe::run_native()
  │
  ▼
  ui/app.rs — LianWorldApp (状态中枢)
  ├── ControlPanel    控制面板（步进/导出/配置）
  ├── CanvasView      画布视图 + 缩略地图
  ├── StatusBar       底部状态栏
  ├── Splash          启动画面（ASCII 渐变）
  ├── AlgoConfig      算法参数配置窗口
  ├── LayerConfig     层级配置窗口
  ├── OverlayConfig   覆盖层配置窗口
  └── Theme           粉蓝白主题系统
  │
  ▼
  generation/ (引擎层)
  ├── pipeline.rs      GenerationPipeline — 步进/回退/重放/缓存
  ├── algorithm.rs     PhaseAlgorithm trait + RuntimeContext
  └── snapshot.rs      WorldSnapshot / export_png
  │
  ▼
  algorithms/ (算法模块层)
  └── biome_division.rs   BiomeDivisionAlgorithm (Phase 1: 环境判定 — 7 子步骤)
      (future: terrain_fill / cave_generation / ore_placement)
  │
  ▼
  core/ (数据层)
  ├── world.rs    World { width, height, tiles }
  ├── biome.rs    BiomeMap (2D 环境网格 + 几何操作)
  ├── block.rs    BlockDefinition
  ├── layer.rs    LayerDefinition + 百分比转行数
  └── color.rs    ColorRgba
  │
  ▼
  config/ (配置加载)
  ├── blocks.rs   blocks.json → BlockConfig
  ├── biome.rs    biome.json  → BiomesConfig
  └── world.rs    world.json  → WorldConfig
  │
  ▼
  rendering/ (渲染)
  ├── canvas.rs     color_lut[256], world_to_color_image
  └── viewport.rs   ViewportState (缩放/平移)
```

## 数据流方向

1. **配置**: JSON → `config/` 加载 → `core/` 构造数据结构
2. **生成**: `pipeline` 调度 `algorithm` → 修改 `World` + `BiomeMap`
3. **渲染**: `World.tiles` → `color_lut` → `ColorImage` → egui 纹理
4. **持久化**: 运行状态 → `WorldSnapshot` → `.lwd` JSON 文件
5. **主题**: `theme.rs` 启动时应用全局粉蓝白配色方案

## 文件清单

| 目录 | 文件 | 行数 | 职责 |
|------|------|------|------|
| src/ | main.rs | 25 | eframe 启动入口，设置 WM_CLASS |
| src/core/ | world.rs | 163 | World / WorldProfile 核心数据 |
| | biome.rs | 270 | BiomeMap 2D 网格 + 几何操作 |
| | block.rs | 26 | BlockDefinition |
| | layer.rs | 41 | LayerDefinition + 百分比转行数 |
| | color.rs | 24 | ColorRgba |
| src/config/ | blocks.rs / biome.rs / world.rs | ~77 | JSON 配置加载 |
| src/generation/ | algorithm.rs | 184 | PhaseAlgorithm trait 定义 |
| | pipeline.rs | 474 | 流水线引擎（含缓存） |
| | snapshot.rs | 149 | 快照模型 + PNG 导出 |
| src/algorithms/ | biome_division.rs | 401 | 环境判定算法模块 |
| src/rendering/ | canvas.rs | 35 | 颜色 LUT 构建 |
| | viewport.rs | 34 | 缩放/平移状态 |
| src/ui/ | app.rs | 837 | 主应用状态中枢 |
| | control_panel.rs | 351 | 左侧控制面板 |
| | canvas_view.rs | 393 | 画布渲染 + 缩略地图 |
| | algo_config.rs | 169 | 算法参数配置窗口 |
| | layer_config.rs | 272 | 层级配置窗口 |
| | overlay_config.rs | 81 | 覆盖层配置窗口 |
| | theme.rs | 166 | 粉蓝白主题配色 |
| | splash.rs | 88 | 启动画面（ASCII 渐变） |
| | status_bar.rs | 30 | 底部状态栏 |
