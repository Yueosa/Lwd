<div align="center">

# LianWorld

</div>

| | |
|-|-|
| ![dashboard](./lwd.png) | ![biome](./biome.png) |
| ![ing](./ing.png) | ![shabox](./shabox.png) |

> 受 Terraria 启发的 2D 世界生成可视化工具 — 用于研究、调试和演示程序化世界生成算法。

## 特性

- **分步生成** — 逐步骤正向/反向执行，实时观察世界构建过程
- **元数据驱动** — 算法自描述参数，GUI 自动生成编辑控件
- **确定性重放** — 相同种子 + 参数 + 世界尺寸 = 完全相同的世界
- **手动种子输入** — 支持十六进制（`0x` 前缀）、纯十六进制、十进制格式
- **存档系统** — `.lwd` 快照导出/导入（只存 seed + params，不存方块）
- **PNG 导出** — 1:1 像素导出世界图像
- **覆盖层可视化** — 环境覆盖色/文字、层级分界线/文字，4 项独立开关
- **粉蓝白主题** — 全局统一配色方案
- **启动画面** — ASCII 艺术 "LIANWORLD" 粉→蓝渐变展示
- **缩略地图** — 画布右下角实时世界缩略图 + 视口指示器

## 技术栈

| 组件 | 技术 |
|------|------|
| 语言 | Rust 2024 Edition |
| GUI | egui / eframe 0.27 |
| 序列化 | serde / serde_json |
| 随机数 | rand 0.8 (StdRng) |
| 噪声 | noise 0.9 |
| 图像 | image 0.25 |
| 文件对话框 | rfd 0.15 |

## 快速开始

```bash
# 构建
cargo build --release

# 运行
cargo run --release
```

## 项目规模

- ~4400 行 Rust 源码
- ~30 个源文件
- 43 种方块 / 6 种环境 / 3 种预设世界尺寸

## 文档导航

| 文档 | 内容 |
|------|------|
| [01 项目概述](docs/01_OVERVIEW.md) | 定位、技术栈、项目规模、视觉特性 |
| [02 系统架构](docs/02_ARCHITECTURE.md) | 模块关系图、数据流、文件清单 |
| [03 引擎-算法机制](docs/03_ENGINE_ALGORITHM.md) | PhaseAlgorithm trait、RuntimeContext、元数据驱动、注册流程 |
| [04 生成管线](docs/04_PIPELINE.md) | 步进模型、确定性重放、增量执行 |
| [05 快照与持久化](docs/05_SNAPSHOT.md) | WorldSnapshot、.lwd 格式、PNG 导出 |
| [06 GUI 工作流](docs/06_GUI.md) | 主题、启动画面、布局、各面板交互、缩略地图 |
| [07 配置系统](docs/07_CONFIG.md) | JSON 配置格式、加载机制、运行时持久化 |
| [08 算法开发指南](docs/08_ALGORITHM_GUIDE.md) | 3 步添加新算法、World API、常见模式 |
| [09 已知问题](docs/09_KNOWN_ISSUES.md) | P2 功能规划、性能优化、已解决问题 |

## 目录结构

```
src/
├── algorithms/       # 生成算法实现
│   └── biome_division.rs
├── assets/           # 编译时嵌入资源
│   ├── blocks.json   # 43 种方块定义
│   ├── biome.json    # 6 种环境定义
│   ├── world.json    # 世界尺寸 + 层级配置
│   └── fonts/        # 子集字体 (CJK + 符号)
├── config/           # 配置加载层
├── core/             # 核心数据结构 (World, Block, Biome, Layer)
├── generation/       # 生成引擎 (Pipeline, Algorithm trait, Snapshot)
├── rendering/        # 渲染工具 (世界→纹理, 颜色 LUT)
├── ui/               # GUI 模块
│   ├── app.rs        # 主应用状态中枢
│   ├── control_panel.rs  # 左侧控制面板
│   ├── canvas_view.rs    # 画布 + 缩略地图
│   ├── theme.rs      # 粉蓝白主题
│   ├── splash.rs     # 启动画面
│   └── ...           # 配置窗口、状态栏
└── main.rs           # 入口
docs/                 # 项目文档 (9 篇)
```
