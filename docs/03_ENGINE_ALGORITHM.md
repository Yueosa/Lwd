# 引擎-算法协作机制

## 核心 Trait: `PhaseAlgorithm`

引擎与算法之间通过唯一的 trait `PhaseAlgorithm` 交互。引擎**零硬编码算法 ID**，完全依赖 `meta()` 返回的元数据。

```rust
pub trait PhaseAlgorithm {
    /// 返回此算法模块的完整元数据
    fn meta(&self) -> PhaseMeta;

    /// 执行指定子步骤（step_index 从 0 开始）
    fn execute(&mut self, step_index: usize, ctx: &mut RuntimeContext) -> Result<(), String>;

    /// 返回当前参数值（JSON，用于序列化/持久化）
    fn get_params(&self) -> serde_json::Value { ... }

    /// 从 JSON 值恢复参数
    fn set_params(&mut self, params: &serde_json::Value) { ... }

    /// 管线重置时调用，清理运行时内部状态
    fn on_reset(&mut self) { ... }
}
```

## 元数据驱动

### PhaseMeta — 算法自描述

```rust
pub struct PhaseMeta {
    pub id: String,          // 唯一标识 (如 "biome_division")
    pub name: String,        // 显示名称 (如 "环境判定")
    pub description: String, // 阶段描述
    pub steps: Vec<StepMeta>,// 子步骤列表（有序）
    pub params: Vec<ParamDef>,// 可调参数定义
}
```

引擎根据 `meta()` 自动：
- 构建步骤列表 UI（步骤名称 / 描述 / 文档链接）
- 生成参数编辑控件（根据 `ParamType` 枚举 → Slider / Checkbox / ComboBox）
- 计算总步骤数、进度条

### ParamType — 参数类型

| 类型 | UI 控件 | 约束 |
|------|---------|------|
| `Float { min, max }` | Slider | 浮点滑块 |
| `Int { min, max }` | Slider | 整数滑块 |
| `Bool` | Checkbox | 布尔开关 |
| `Text` | TextEdit | 自由文本 |
| `Enum { options }` | ComboBox | 下拉选择 |

## RuntimeContext — 执行上下文

算法执行时，引擎通过 `RuntimeContext` 提供所有必要资源：

```rust
pub struct RuntimeContext<'a> {
    pub world: &'a mut World,                         // 方块数据（读写）
    pub profile: &'a WorldProfile,                    // 世界配置（只读）
    pub blocks: &'a [BlockDefinition],                // 方块定义表
    pub biomes: &'a [BiomeDefinition],                // 环境定义表
    pub rng: &'a mut StdRng,                          // 确定性 RNG
    pub biome_map: &'a mut Option<BiomeMap>,           // 环境地图（共享）
    pub shared: &'a mut HashMap<String, Box<dyn Any>>, // 通用共享状态
}
```

### 通用共享状态 (`shared`)

跨步骤/跨阶段的中间数据通过 `shared` 容器传递：

```rust
// 写入（Phase 2 地形生成阶段）
ctx.shared.insert("heightmap".into(), Box::new(vec![0u32; w * h]));

// 读取（Phase 3 洞穴生成阶段）
let hm = ctx.shared.get("heightmap")
    .and_then(|v| v.downcast_ref::<Vec<u32>>())
    .expect("heightmap 未初始化");
```

管线重置时 `shared` 自动清空。

## 注册流程

在 `src/generation/mod.rs` 的 `build_pipeline()` 中注册：

```rust
pub fn build_pipeline(seed: u64, biomes: Vec<BiomeDefinition>) -> GenerationPipeline {
    let mut pipeline = GenerationPipeline::new(seed, biomes.clone());

    // Phase 1
    pipeline.register(Box::new(BiomeDivisionAlgorithm::new(&biomes)));
    // Phase 2+
    // pipeline.register(Box::new(TerrainFillAlgorithm::new(&biomes)));

    pipeline
}
```

## 确定性 RNG

- 主种子 `seed` 在管线创建时设定
- 每个子步骤的 RNG 由 `derive_step_seed(master, flat_index, world_width, world_height)` 派生
- 世界尺寸参与种子派生：同一 seed + 不同世界尺寸 = 不同的生成结果（与泰拉瑞亚行为一致）
- 相同 seed + 相同参数 + 相同世界尺寸 = 完全相同的世界（无论步进顺序）
- 回退通过「从头重放」实现，利用确定性保证结果一致

## 生命周期回调

| 回调 | 时机 | 用途 |
|------|------|------|
| `execute()` | 每步执行 | 核心生成逻辑 |
| `on_reset()` | 管线重置/回退时 | 清理运行时内部状态 |
| `get_params()` | 导出存档/配置面板 | 序列化参数 |
| `set_params()` | 导入存档/配置面板 | 恢复参数 |
