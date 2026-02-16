# GUI 工作流

## 整体布局

```
  控制面板            画布视图
  (Side Panel)       (Central Panel)

  · 标题 (✿ Lian World)
  · 世界尺寸          棋盘格背景
  · 种子输入          ├ 世界纹理 (zoom + pan)
  · 生成进度          ├ Biome 覆盖层 (半透明)
  · 步进按钮          ├ Biome 标签
  · 步骤列表          ├ 层级分界线
  · 操作按钮          ├ 层级文字标签
  · 导出/导入         └ 缩略地图 (右下角)
  · 缩放控制
  · 配置

  状态栏 (底部)
  状态 | hover | Step 1.2 (3/7) | 4200×1200 | Seed | FPS | 内存
```

## 主题系统 (`theme.rs`)

全局采用粉蓝白配色方案，启动时通过 `apply_theme()` 应用到 egui Visuals：

- **粉色系**：标题、当前步骤高亮、进度条
- **蓝色系**：按钮悬停、链接、进度条终点
- **白色**：主文字
- **深色背景**：`#1A1A2E` 系面板底色

## 启动画面 (`splash.rs`)

首次打开时显示 ASCII 艺术 "LIANWORLD"，粉→蓝逐行渐变。
包含副标题和闪烁提示文字。生成操作启动后切换到画布视图（通过 `has_started_generation` 标志控制）。

## 每帧更新流程 (`update()`)

1. **FPS 限制** → 空闲状态下 `request_repaint_after(16ms)`，限制 60FPS
2. **检查世界尺寸变更** → 重建 World / Pipeline
3. **刷新纹理** → 仅 `texture_dirty` 时重建
4. **渲染控制面板** → 收集 `ControlAction`（纯 UI 输出，不触碰 Pipeline）
5. **弹出窗口** → 算法配置、层级配置、可视化配置（若已打开）
6. **处理动作** → `handle_action()` 根据 `ControlAction` 操作 Pipeline
7. **增量执行** → `running_to_end` 时每帧执行 3 子步骤，每 5 帧刷新一次纹理
8. **渲染状态栏** → 操作反馈 + hover + 步骤进度 + 世界尺寸 + Seed + FPS + 内存
9. **渲染画布** → 世界 + 覆盖层 + 缩略地图 + 交互

## 控制面板 (`control_panel.rs`)

### ControlAction 模式

控制面板**不直接操作 Pipeline**，而是产生一个 `ControlAction` 结构体（16 个 bool 字段），由 `app.rs` 的 `handle_action()` 统一消费。

### UI 区块（从上到下）

| 区块 | 内容 |
|------|------|
| 标题 | "✿ Lian World"（居中，粉色） |
| 世界尺寸 | Radio: 小 / 中 / 大 |
| 种子输入 | TextEdit（支持十六进制 `0x` 前缀 / 纯十六进制 / 十进制）+ 确认按钮，Enter 亦可应用 |
| 生成进度 | 粉→蓝渐变 ProgressBar + "X / Y 步" |
| 步进控制 | ⏮ 大步后退 · ◂ 小步后退 · ▸ 小步前进 · ⏭ 大步前进 |
| 步骤列表 | 两级折叠列表，Phase → SubStep，粉蓝白状态标记 |
| 操作按钮 | ▶ 一键生成 · ↻ 重新初始化 · ◈ 算法配置 |
| 导出/导入 | ▣ PNG 导出 · □ .lwd 导出 · ■ .lwd 导入 |
| 缩放控制 | ＋ 放大 / ＆ 缩小 / ↺ 重置 |
| 配置 | ◉ 可视化配置 · ▧ 层级配置 |

### 步骤列表状态

- ● **粉色** — 已完成
- ◆ **蓝色** — 当前步骤
- ○ **灰色** — 待执行

Hover 显示步骤描述和文档链接。

## 算法配置窗口 (`algo_config.rs`)

**完全元数据驱动**，不硬编码任何具体参数。

流程：
1. 调用 `algorithm.meta()` → 获取 `PhaseMeta` 的 `params: Vec<ParamDef>`
2. 调用 `algorithm.get_params()` → 获取当前参数值 (`serde_json::Value`)
3. 对每个 `ParamDef`，根据 `ParamType` 自动生成对应控件
4. 参数修改后立即写回 `algorithm.set_params()`

可用操作：
- **重新执行当前步骤** — 触发 replay（回退到阶段起始 → 重放到当前位置）
- **重置为默认值** — 恢复 `ParamDef.default`

## 画布视图 (`canvas_view.rs`)

### 渲染层次

| 层 | 内容 | 条件 |
|----|------|------|
| 1 | 棋盘格背景 | 始终显示 |
| 2 | 世界纹理 | 始终显示 |
| 3 | Biome 覆盖层 | `show_biome_color` |
| 4 | Biome 标签 | `show_biome_labels` |
| 5 | 层级分界线 | `show_layer_lines` |
| 6 | 层级文字标签 | `show_layer_labels` |
| 7 | 缩略地图 | 始终显示（右下角） |

### 缩略地图

- 位于画布右下角，最大 180×110 像素
- 保持世界宽高比
- 蓝色矩形指示当前视口位置
- 半透明深色背景 + 白色边框

### 交互

- **拖拽平移** — 鼠标左键拖动
- **滚轮缩放** — 以光标位置为锚点，~5%/tick，范围 0.05x – 20x
- **Hover** — 状态栏显示: `方块名(ID:N) @ (x, y) | 环境名·层级名`

## 可视化配置窗口 (`overlay_config.rs`)

独立弹窗，通过 "◉ 可视化配置" 按钮打开。提供 4 个独立开关：

| 开关 | 说明 |
|------|------|
| 显示环境覆盖色 | 半透明 Biome 区域颜色 |
| 显示环境文字标签 | Biome 名称文字 |
| 显示层级分界线 | 层级边界水平线 |
| 显示层级文字标签 | 层级名称文字 |

底部提供"全部开启"和"全部关闭"快捷按钮。开关状态自动持久化到 `generation.runtime.json`。

## 层级配置窗口 (`layer_config.rs`)

编辑 5 个世界层级（space / surface / underground / cavern / hell）的垂直分布。

### 双模式

| 模式 | 编辑内容 | 只读显示 |
|------|----------|----------|
| 百分比模式 | start/end 百分比 | 行数 |
| 绝对行数模式 | start/end 行数 | 百分比 |

### 智能相邻层对齐

修改某层 `end_percent` 时，若下一层 `start_percent` 原本与之相同，则自动同步调整。

### 操作

- **恢复默认** — 硬编码 (space 0-5%, surface 5-25%, underground 25-35%, cavern 35-80%, hell 80-100%)
- **保存配置** — 写入 `generation.runtime.json` 的 `layers` 字段

## 状态栏 (`status_bar.rs`)

单行布局：

```
状态: {操作反馈} | {方块名(ID:N) @ (x,y) | 环境·层级} | Step 1.2 (3/7) | 4200×1200 | Seed: 00112233AABBCCDD | FPS: 60 | 内存: ~12MB
```

| 字段 | 来源 | 说明 |
|------|------|------|
| 状态 | `last_status` | 持久操作反馈（"已导出 PNG" 等） |
| hover | `hover_status` | 方块名 + ID + 坐标 + 环境名·层级名 |
| Step | `pipeline.current_step_display_id()` | 当前步骤编号 + 执行进度 |
| 尺寸 | `world.width × height` | 当前世界尺寸 |
| Seed | `pipeline.seed()` | 16 位十六进制显示 |
| FPS | `1.0 / stable_dt` | 帧率（空闲限 60FPS） |
| 内存 | `width × height × 4 / 1MB` | 简单估算 |
