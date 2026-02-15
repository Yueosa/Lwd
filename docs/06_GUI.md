# GUI 工作流

## 整体布局

```
┌──────────────┬──────────────────────────────────────┐
│              │                                      │
│  控制面板     │            画布视图                    │
│ (Side Panel) │        (Central Panel)               │
│              │                                      │
│  · 世界尺寸   │   棋盘格背景                          │
│  · 生成进度   │   ├ 世界纹理 (zoom + pan)              │
│  · 步进按钮   │   ├ Biome 覆盖层 (半透明)              │
│  · 步骤列表   │   ├ Biome 标签                        │
│  · 操作按钮   │   └ 层级分界线 + 标签                   │
│  · 导出/导入  │                                      │
│  · 视图控制   │                                      │
│              │                                      │
├──────────────┴──────────────────────────────────────┤
│  状态栏: 操作信息 | 悬停信息 | FPS | 内存                │
└─────────────────────────────────────────────────────┘
```

## 每帧更新流程 (`update()`)

1. **检查世界尺寸变更** → 重建 World / Pipeline
2. **刷新纹理** → 仅 `texture_dirty` 时重建
3. **渲染控制面板** → 收集 `ControlAction`（纯 UI 输出，不触碰 Pipeline）
4. **弹出窗口** → 算法配置、层级配置（若已打开）
5. **处理动作** → `handle_action()` 根据 `ControlAction` 操作 Pipeline
6. **增量执行** → `running_to_end` 时每帧执行 3 子步骤
7. **渲染状态栏** → FPS + 内存 + 操作反馈 + hover
8. **渲染画布** → 世界 + 覆盖层 + 交互

## 控制面板 (`control_panel.rs`)

### ControlAction 模式

控制面板**不直接操作 Pipeline**，而是产生一个 `ControlAction` 结构体（16 个 bool 字段），由 `app.rs` 的 `handle_action()` 统一消费。

### UI 区块（从上到下）

| 区块 | 内容 |
|------|------|
| 标题 | "🗺 Lian World" |
| 世界尺寸 | Radio: 小 / 中 / 大 |
| 生成进度 | ProgressBar + "X / Y 步" |
| 步进控制 | ◀◀ 大步后退 · ◀ 小步后退 · ▶ 小步前进 · ▶▶ 大步前进 |
| 步骤列表 | 两级折叠列表，Phase → SubStep，颜色状态标记 |
| 操作按钮 | 一键生成 · 重新初始化 · 算法配置 |
| 导出/导入 | PNG · .lwd 导出 · .lwd 导入 |
| 缩放控制 | + / - / 重置 |
| 可视化图层 | Biome 覆盖 · 层级覆盖 · 配置层级 |

### 步骤列表状态

- ✓ **黄色** — 已完成
- ▶ **绿色** — 当前步骤
- · **灰色** — 待执行

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
| 3 | Biome 覆盖层 | `show_biome_overlay` |
| 4 | Biome 标签 | `show_biome_overlay` |
| 5 | 层级分界线 + 标签 | `show_layer_overlay` |

### 交互

- **拖拽平移** — 鼠标左键拖动
- **滚轮缩放** — 以光标位置为锚点，~5%/tick，范围 0.05x ‒ 20x
- **Hover** — 屏幕坐标 → 世界坐标，状态栏显示方块名称和坐标

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
状态: {操作反馈} | {方块名称 @ (x, y)} | FPS: 60.0 | 内存: ~12MB
```

- `last_status` — 持久操作反馈（"已导出 PNG"、"已前进 1 步" 等）
- `hover_status` — 瞬时 hover 信息（鼠标离开画布时清空）
- FPS — `1.0 / stable_dt`
- 内存 — `width × height × 4 / 1MB` 估算
