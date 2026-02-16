# 算法开发指南

## 快速开始：添加一个新算法

只需 3 步即可添加新的生成阶段。

### 第 1 步：实现 `PhaseAlgorithm` trait

创建 `src/algorithms/my_algorithm.rs`：

```rust
use crate::generation::algorithm::*;
use serde::{Deserialize, Serialize};

/// 参数结构体（可序列化）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyAlgorithmParams {
    pub intensity: f64,
    pub enabled: bool,
}

impl Default for MyAlgorithmParams {
    fn default() -> Self {
        Self { intensity: 0.5, enabled: true }
    }
}

pub struct MyAlgorithm {
    params: MyAlgorithmParams,
    // 运行时内部状态（非参数，不需要持久化）
    cache: Option<Vec<u32>>,
}

impl MyAlgorithm {
    pub fn new() -> Self {
        Self {
            params: MyAlgorithmParams::default(),
            cache: None,
        }
    }
}

impl PhaseAlgorithm for MyAlgorithm {
    fn meta(&self) -> PhaseMeta {
        PhaseMeta {
            id: "my_algorithm".into(),
            name: "我的算法".into(),
            description: "示例生成阶段".into(),
            steps: vec![
                StepMeta {
                    display_index: 1,
                    name: "初始化".into(),
                    description: "准备数据".into(),
                    doc_url: None,
                },
                StepMeta {
                    display_index: 2,
                    name: "填充".into(),
                    description: "填充方块".into(),
                    doc_url: None,
                },
            ],
            params: vec![
                ParamDef {
                    key: "intensity".into(),
                    name: "强度".into(),
                    description: "生成强度".into(),
                    param_type: ParamType::Float { min: 0.0, max: 1.0 },
                    default: serde_json::json!(0.5),
                },
                ParamDef {
                    key: "enabled".into(),
                    name: "启用".into(),
                    description: "是否启用".into(),
                    param_type: ParamType::Bool,
                    default: serde_json::json!(true),
                },
            ],
        }
    }

    fn execute(&mut self, step_index: usize, ctx: &mut RuntimeContext) -> Result<(), String> {
        match step_index {
            0 => self.step_init(ctx),
            1 => self.step_fill(ctx),
            _ => Err(format!("未知步骤: {step_index}")),
        }
    }

    fn get_params(&self) -> serde_json::Value {
        serde_json::to_value(&self.params).unwrap_or_default()
    }

    fn set_params(&mut self, params: &serde_json::Value) {
        if let Ok(p) = serde_json::from_value(params.clone()) {
            self.params = p;
        }
    }

    fn on_reset(&mut self) {
        self.cache = None; // 清理运行时状态
    }
}

impl MyAlgorithm {
    fn step_init(&mut self, ctx: &mut RuntimeContext) -> Result<(), String> {
        // 使用 World 安全 API
        let w = ctx.world.width as usize;
        let h = ctx.world.height as usize;

        // 使用确定性 RNG
        let val: f64 = ctx.rng.gen();

        // 存入跨步骤共享状态
        ctx.shared.insert("my_data".into(), Box::new(vec![0u32; w]));
        Ok(())
    }

    fn step_fill(&mut self, ctx: &mut RuntimeContext) -> Result<(), String> {
        // 读取共享状态
        let data = ctx.shared.get("my_data")
            .and_then(|v| v.downcast_ref::<Vec<u32>>())
            .ok_or("共享状态 my_data 未找到")?;

        // 安全写入方块
        for x in 0..ctx.world.width {
            ctx.world.set(x, 100, 1); // 设置 (x, 100) 为方块 ID 1 (泥土)
        }

        // 或批量填充
        ctx.world.fill_rect(10, 50, 20, 60, 3); // 矩形填充石块

        Ok(())
    }
}
```

### 第 2 步：注册模块

在 `src/algorithms/mod.rs` 中导出：

```rust
pub mod my_algorithm;
```

### 第 3 步：注册到管线

在 `src/generation/mod.rs` 的 `build_pipeline()` 中：

```rust
use crate::algorithms::my_algorithm::MyAlgorithm;

pipeline.register(Box::new(MyAlgorithm::new()));
```

完成！引擎自动：
- 在步骤列表中显示新阶段
- 生成参数编辑 UI（根据 `meta().params`）
- 纳入步进/回退/重放/快照/导出流程

## World 安全 API

| 方法 | 签名 | 说明 |
|------|------|------|
| `get(x, y)` | `Option<u8>` | 安全读取，越界返回 `None` |
| `get_or_air(x, y)` | `u8` | 越界返回 AIR (0) |
| `set(x, y, id)` | `()` | 安全写入，越界静默忽略 |
| `fill_rect(x0, y0, x1, y1, id)` | `()` | 矩形区域填充 |
| `fill_column(x, ys, ye, id)` | `()` | 纵列填充 |
| `for_each_in_rows(ys, ye, f)` | `()` | 逐行回调（高效批量操作） |
| `in_bounds(x, y)` | `bool` | 坐标有效性检查 |

## RuntimeContext 可用资源

| 字段 | 类型 | 说明 |
|------|------|------|
| `world` | `&mut World` | 方块数据（读写） |
| `profile` | `&WorldProfile` | 世界配置（只读） |
| `blocks` | `&[BlockDefinition]` | 方块定义表 |
| `biomes` | `&[BiomeDefinition]` | 环境定义表 |
| `rng` | `&mut StdRng` | 确定性 RNG（每步唯一种子） |
| `biome_map` | `&mut Option<BiomeMap>` | 环境地图（跨步骤共享） |
| `shared` | `&mut HashMap<String, Box<dyn Any>>` | 通用共享状态 |

## 注册约束

- 算法注册顺序 = 执行顺序
- `meta().id` 必须唯一
- `meta().steps` 数量决定该阶段的子步骤数
- `on_reset()` 必须清理所有运行时内部状态（否则回退重放会产生不一致）

## 常见模式

### 读取 BiomeMap

```rust
if let Some(ref bmap) = ctx.biome_map {
    let biome_id = bmap.get(x, y);
    // 根据环境做不同处理
}
```

### 使用 noise crate

```rust
use noise::{NoiseFn, Perlin};

let perlin = Perlin::new(ctx.rng.gen());
let val = perlin.get([x as f64 / 50.0, y as f64 / 50.0]);
```

### 查询方块属性

```rust
let block_id = ctx.world.get_or_air(x, y);
let block = &ctx.blocks[block_id as usize];
// block.name, block.category 等
```
