# Lwd 文档

> **Lian World** — Terraria 风格 2D 世界生成可视化工具
>
> 版本 1.1.0 · Rust Edition 2024 · [GitHub](https://github.com/Yueosa/Lwd)

---

## 写在前面

Lwd 是 Yueosa 的个人项目，灵感来源于 Terraria 的世界生成过程。它将整个世界的诞生过程拆解为可逐步执行、回退和可视化观察的步骤序列，你可以在这个过程中调整参数、查看几何形状的布局，并导出最终结果。

如果你是用户，从 **UI 使用手册** 开始。如果你想编写自己的生成算法，请阅读 **算法开发指南**。

---

## 文档导航

| 文档 | 说明 |
|------|------|
| [modules.md](modules.md) | **模块总览** — 引擎各模块的职责、角色和实现思路 |
| [ui_guide.md](ui_guide.md) | **UI 使用手册** — 每个面板、按钮和交互操作的详细说明 |
| [algorithm_guide.md](algorithm_guide.md) | **算法开发指南** — 资产文件、引擎 API、几何系统、开发教程 |
| [known_issues.md](known_issues.md) | **已知问题** — 当前限制、性能特征和待实现功能 |

---

## 快速参考

**源码结构：**

```
src/
├── core/          数据模型（World, Block, Biome, Layer, Geometry）
├── config/        JSON 配置加载
├── assets/        嵌入式资源（blocks.json, biome.json, world.json, 字体）
├── generation/    管线引擎（Pipeline, PhaseAlgorithm, Optimizer, Snapshot）
├── rendering/     渲染层（CPU canvas, GL canvas, Viewport）
├── storage/       持久化（runtime.json, 性能日志）
├── ui/            全部界面（12 个模块）
└── algorithms/    可插拔生成算法
```

**技术栈：** egui 0.27 + glow 0.13（OpenGL 3.1+）· rayon 1.10 · serde · rand · image · rfd

**规模：** 51 个源文件 · ~8,700 行引擎代码 · 43 种方块 · 10 种环境 · 5 个层级

---
