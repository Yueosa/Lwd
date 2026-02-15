# 已知问题与未来规划

## P2 — 功能待实现

### 多层世界 (Multi-Layer World)

当前 World 只有一层 `tiles: Vec<u8>`，对应 Terraria 的背景墙 / 前景方块 / 液体等多层结构尚未支持。

**规划方向**：
- `World` 添加 `layers: Vec<Vec<u8>>`，每层独立 `width × height`
- Canvas 渲染时按层级叠加
- 算法通过 `ctx.world.layer(index)` 访问

### 自定义世界尺寸

`world.json` 中已预留 `"custom": { "width": null, "height": null }` 但 UI 尚未实现自定义输入。

### 检查点（Checkpoint）

当前回退通过从零重放实现，随着步骤增多（未来可能 50+ 步），重放延迟将明显增长。

**规划方向**：
- 每 N 步保存一次 World 完整快照（内存检查点）
- 回退时从最近检查点开始重放
- 内存 / 时间权衡可由用户配置

### 版本迁移

快照 `.lwd` 文件有 `version` 字段，但尚未实现跨版本迁移逻辑。

**规划方向**：
- 维护 `migrate_v1_to_v2()` 等函数链
- 加载时自动应用迁移直到当前版本

### 热重载配置

`blocks.json` / `biome.json` / `world.json` 通过 `include_str!()` 编译时嵌入，修改后需重新编译。

**规划方向**：
- 开发模式：文件系统监听 + 热重载
- 发布模式：维持编译时嵌入

## 性能相关

### 全纹理重建

每次世界数据变更后需重建整个 `ColorImage`（大世界 8400×2400 = ~80MB RGBA）。

> **部分缓解**：增量执行（`running_to_end`）期间，通过 `inc_frame_counter` 每 5 帧才刷新一次纹理，大幅减少 GPU 上传频率。但单次刷新仍为全帧重建。

**优化方向**：
- 脏区域追踪：只重建变更的矩形区域
- GPU 纹理更新：直接修改纹理子区域而非整帧替换

### ~~phase_info_list 每帧分配~~ ✅ 已解决

`phase_info_list()` 现已通过 `cached_phase_info` + dirty flag 实现缓存，仅在步骤变化时重建。方法签名变为 `&mut self -> &[PhaseInfo]`。`total_sub_steps()` 和 `executed_sub_steps()` 通过 `step_counts` / `total_steps_cache` 实现 O(1) 查询。

### 增量执行帧预算

当前固定 `STEPS_PER_FRAME = 3`，未根据帧耗时自适应。

**优化方向**：
- 测量每帧剩余时间，动态调整步数
- 帧预算目标 ≤ 16ms（60 FPS）

## 测试

当前无自动化测试。

**规划方向**：
- 单元测试：World 安全 API、确定性 RNG 派生、快照序列化
- 集成测试：Pipeline 完整重放一致性（seed A → 运行到底 = seed A → 步进到底 = seed A → 回退再前进到底）
- 模糊测试：随机种子大规模生成不 panic

## 工程改进

### 警告清理

编译存在 ~18 个 warning（未使用的 import/field），应逐步清理。

### CI/CD

无持续集成配置。

**规划方向**：
- GitHub Actions：`cargo check` + `cargo test` + `cargo clippy`
- 自动发布构建（Linux / Windows / macOS）

### 日志

无结构化日志，调试依赖 `println!`。

**规划方向**：
- 引入 `tracing` crate
- 生成过程日志（每步耗时、参数摘要）
