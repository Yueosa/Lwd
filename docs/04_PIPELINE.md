# 生成管线 (Pipeline)

## 结构概览

`GenerationPipeline` 是生成过程的核心调度器。它维护：

- **阶段列表 (`algorithms`)** — 注册的 `Box<dyn PhaseAlgorithm>` 有序数组
- **步骤数缓存 (`step_counts`)** — 每个阶段的子步骤数 `Vec<usize>`，注册时缓存
- **总步骤数缓存 (`total_steps_cache`)** — 所有阶段子步骤总数，`total_sub_steps()` O(1) 查询
- **当前执行位置 (`current_phase` / `current_sub`)** — 指向下一个要执行的子步骤
- **主种子 (`seed`)** — 全局 RNG 根种子
- **共享状态 (`shared_state`)** — `HashMap<String, Box<dyn Any>>`（跨阶段传递）
- **PhaseInfo 缓存** — `cached_phase_info` + dirty flag，`phase_info_list()` 仅在步骤变化时重建

## 步进模型

### 正向步进 (`step_forward_sub`)

每次调用执行**1 个子步骤**：

```
flat_index: 0 →  1 →  2 →  3 → ... → total_steps-1
Phase:      |---Phase 0---|----Phase 1----|...
Step:       s0   s1   s2    s0    s1  ...
```

1. 通过 `flat_index` 计算当前 phase_index + step_index
2. 以 `derive_step_seed(seed, flat_index, world_width, world_height)` 创建确定性 RNG（世界尺寸参与派生）
3. 构建 `RuntimeContext`
4. 调用 `algorithm.execute(step_index, &mut ctx)`
5. `flat_index += 1`

### 反向步进 (`step_backward_sub` / `step_backward_phase`)

由于世界写入不可逆，回退通过**重放**实现：

```
当前 flat_index = 15, 目标 = 14:
1. 记录目标 target = 14
2. 重置世界 + shared_state + 调用所有 on_reset()
3. 从 flat_index=0 重新执行到 flat_index=14
```

**`step_backward_phase` 语义**（修正后）：
- 若当前不在阶段起始位置 → 回退到**当前阶段起始**
- 若已在阶段起始位置 → 回退到**上一阶段起始**
- 例：Phase 1 步骤 3 → Phase 1 步骤 0 → Phase 0 步骤 0

### `replay_to_flat(target)`

回退/跳转到任意步骤的核心方法：

1. 清空 world 数据
2. 清空 shared_state
3. 逐算法调用 `on_reset()`
4. 从 0 顺序执行到 target

## 增量执行模型

为避免 `run_all()` 阻塞 UI，引入增量执行：

```rust
// app.rs 中 update() 每帧执行
const STEPS_PER_FRAME: usize = 3;

if self.running_to_end && !self.pipeline.is_complete() {
    for _ in 0..STEPS_PER_FRAME {
        if self.pipeline.is_complete() { break; }
        self.pipeline.step_forward_sub(&mut self.world, &self.world_profile, &self.blocks)?;
    }
    // 增量执行期间，每 5 帧才刷新一次纹理，大幅减少 GPU 上传开销
    self.inc_frame_counter += 1;
    let should_refresh = self.pipeline.is_complete() || self.inc_frame_counter % 5 == 0;
    if should_refresh {
        self.texture_dirty = true;
    }
    ctx.request_repaint();
}
```

- 每帧最多执行 3 个子步骤（可调）
- **纹理增量刷新**：通过 `inc_frame_counter` 计数，仅每 5 帧刷新一次纹理，减少 GPU 上传开销
- 通过 `ctx.request_repaint()` 保持帧循环活跃
- 状态栏显示 "正在生成… X/Y" 进度
- 导入 `.lwd` 存档时同样使用增量模式

## 确定性保证

| 要素 | 机制 |
|------|------|
| RNG 种子 | `derive_step_seed(master, flat_index, world_width, world_height)` — 与执行历史无关，世界尺寸参与派生 |
| 执行顺序 | 扁平索引严格递增 |
| 参数 | 序列化在 WorldSnapshot 中 |
| 回退 | 从零重放（同 seed + 同参数 + 同尺寸 = 同结果） |

只要 seed 和参数相同，无论从头生成还是回退重放，**结果完全一致**。
