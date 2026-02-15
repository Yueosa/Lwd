# 架构全景

## 模块关系图

```
┌─────────────────────────────────────────────────────┐
│                     main.rs                          │
│              eframe::run_native()                    │
└──────────────────────┬──────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────┐
│                  ui/app.rs                           │
│            LianWorldApp (状态中枢)                    │
│  ┌──────────┐ ┌──────────┐ ┌────────────────┐       │
│  │ControlPanel│ │CanvasView│ │AlgoConfig     │       │
│  │ LayerConfig│ │StatusBar │ │               │       │
│  └──────┬───┘ └────┬─────┘ └──────┬─────────┘       │
└─────────┼──────────┼──────────────┼─────────────────┘
          │          │              │
          ▼          ▼              ▼
┌─────────────────────────────────────────────────────┐
│              generation/ (引擎层)                     │
│  ┌────────────────┐  ┌──────────────────────────┐   │
│  │ pipeline.rs     │  │ algorithm.rs (Trait定义)  │   │
│  │ GenerationPipeline│ │ PhaseAlgorithm           │   │
│  │ 步进/回退/重放   │  │ RuntimeContext            │   │
│  └───────┬────────┘  └──────────┬───────────────┘   │
│          │                      │                    │
│  ┌───────▼──────────────────────▼───────────────┐   │
│  │              snapshot.rs                       │   │
│  │     WorldSnapshot / export_png                │   │
│  └──────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────┐
│           algorithms/ (算法模块层)                     │
│  ┌──────────────────────────────────────────────┐   │
│  │ biome_division.rs                             │   │
│  │ BiomeDivisionAlgorithm: PhaseAlgorithm        │   │
│  │ (Phase 1: 环境判定 — 7 个子步骤)               │   │
│  └──────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────┐   │
│  │ (future) terrain_fill.rs                      │   │
│  │ (future) cave_generation.rs                   │   │
│  │ (future) ore_placement.rs                     │   │
│  └──────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────┐
│                core/ (数据层)                         │
│  world.rs   — World { width, height, tiles }         │
│  biome.rs   — BiomeMap (2D 环境网格)                  │
│  block.rs   — BlockDefinition (方块定义)              │
│  layer.rs   — LayerDefinition (层级定义)              │
│  color.rs   — ColorRgba (颜色适配)                    │
└─────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────┐
│              config/ (配置加载)                       │
│  blocks.rs  — blocks.json → BlockConfig              │
│  biome.rs   — biome.json  → BiomesConfig             │
│  world.rs   — world.json  → WorldConfig              │
└─────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────┐
│            rendering/ (渲染)                          │
│  canvas.rs   — color_lut[256], world_to_color_image  │
│  viewport.rs — ViewportState (缩放/平移)              │
└─────────────────────────────────────────────────────┘
```

## 数据流方向

1. **配置**: JSON → `config/` 加载 → `core/` 构造数据结构
2. **生成**: `pipeline` 调度 `algorithm` → 修改 `World` + `BiomeMap`
3. **渲染**: `World.tiles` → `color_lut` → `ColorImage` → egui 纹理
4. **持久化**: 运行状态 → `WorldSnapshot` → `.lwd` JSON 文件

## 文件清单

| 目录 | 文件 | 行数 | 职责 |
|------|------|------|------|
| src/ | main.rs | 25 | eframe 启动入口 |
| src/core/ | world.rs | 160+ | World / WorldProfile 核心数据 |
| | biome.rs | 270 | BiomeMap 2D 网格 + 几何操作 |
| | block.rs | 26 | BlockDefinition |
| | layer.rs | 42 | LayerDefinition + 百分比转行数 |
| | color.rs | 24 | ColorRgba |
| src/config/ | blocks.rs / biome.rs / world.rs | ~80 | JSON 配置加载 |
| src/generation/ | algorithm.rs | 170 | PhaseAlgorithm trait 定义 |
| | pipeline.rs | 450 | 流水线引擎 |
| | snapshot.rs | 150 | 快照模型 + PNG 导出 |
| src/algorithms/ | biome_division.rs | 340 | 环境判定算法模块 |
| src/rendering/ | canvas.rs | 35 | 颜色 LUT 构建 |
| | viewport.rs | 35 | 缩放/平移状态 |
| src/ui/ | app.rs | 680 | 主应用状态中枢 |
| | control_panel.rs | 260 | 左侧控制面板 |
| | algo_config.rs | 170 | 算法参数配置窗口 |
| | canvas_view.rs | 300 | 中央画布渲染 |
| | layer_config.rs | 273 | 层级配置窗口 |
| | status_bar.rs | 15 | 底部状态栏 |
