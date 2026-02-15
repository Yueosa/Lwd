# 快照与持久化

## 设计哲学

> **不存方块数据，只存 seed + 参数。**

由于确定性 RNG 保证相同 seed + 相同参数 = 完全相同的世界，快照仅需记录：
- 主种子
- 世界尺寸
- 层级配置
- 各算法参数

导入时对 pipeline 执行完整重放即可还原方块数据。

## WorldSnapshot 结构

```rust
pub struct WorldSnapshot {
    pub version: u32,                       // 存档格式版本 (当前 = 1)
    pub seed: u64,                          // 主种子
    pub world_size: String,                 // "small" / "medium" / "large"
    pub layers: HashMap<String, LayerOverride>, // 层级覆盖
    pub algorithms: Vec<AlgorithmState>,    // 各算法参数
    pub timestamp: u64,                     // 导出时间 (Unix 秒)
}
```

## `.lwd` 文件格式

扩展名 `.lwd`（**L**ian **W**orld **D**ata），内容为 JSON pretty-print：

```json
{
  "version": 1,
  "seed": 12345,
  "world_size": "medium",
  "layers": {
    "dirt": { "start_percent": 0, "end_percent": 35 },
    "stone": { "start_percent": 35, "end_percent": 100 }
  },
  "algorithms": [
    { "algorithm_id": "biome_division", "params": { "ocean_ratio": 0.15, ... } }
  ],
  "timestamp": 1718234567
}
```

## 导出流程

1. 用户点击 "导出存档"
2. `rfd::FileDialog` 弹出系统文件选择器（过滤器 `*.lwd`）
3. `WorldSnapshot::collect()` 从当前运行状态收集数据
4. `snapshot.save_lwd(path)` 序列化写入

## 导入流程

1. 用户点击 "导入存档"
2. `rfd::FileDialog` 弹出选择器
3. `WorldSnapshot::load_lwd(path)` 读取并验证版本号
4. 恢复参数：
   - 重建世界尺寸 + 层级配置
   - 逐算法调用 `set_params()`
   - 设定 seed
5. 进入增量执行模式（`running_to_end = true`）完成重放

## PNG 导出

`export_png()` 将方块数据映射为 RGBA 图像：

- 每个方块 → 查询 `color_lut[block_id]` 获得 `Color32`
- 输出 RGBA 8-bit PNG
- 分辨率 = 世界尺寸（像素 1:1 方块）

## 版本兼容性

- `SNAPSHOT_VERSION = 1`
- 加载时检查版本号：`snapshot.version > SNAPSHOT_VERSION` → 拒绝加载
- 未来版本升级需提供迁移逻辑（P2 待实现）
