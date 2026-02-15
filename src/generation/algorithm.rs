//! # 算法模块接口定义
//!
//! 本模块定义了引擎与算法之间的契约。每个算法模块实现 [`PhaseAlgorithm`] trait，
//! 通过 [`PhaseMeta`] 向引擎声明自身的元数据（名称、描述、子步骤列表、可调参数），
//! 引擎根据这些元数据自动构建 UI 面板和流水线。
//!
//! ## 设计原则
//!
//! - **引擎不感知算法内容**：引擎只读取 `meta()` 返回的元数据，不硬编码任何算法 ID。
//! - **算法自描述**：每个算法模块完整定义自己的步骤列表和参数 schema。
//! - **参数持久化**：引擎通过 `get_params()`/`set_params()` 做序列化，算法无需关心 I/O。

use std::any::Any;
use std::collections::HashMap;

use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};

use crate::core::biome::{BiomeDefinition, BiomeMap};
use crate::core::block::BlockDefinition;
use crate::core::world::{World, WorldProfile};

// ═══════════════════════════════════════════════════════════
// 元数据结构 —— 算法用这些结构向引擎描述自身
// ═══════════════════════════════════════════════════════════

/// 参数类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParamType {
    /// 浮点数，带范围 [min, max]
    Float { min: f64, max: f64 },
    /// 整数，带范围 [min, max]
    Int { min: i64, max: i64 },
    /// 布尔值
    Bool,
    /// 字符串（自由文本）
    Text,
    /// 枚举选择，可选项列表
    Enum { options: Vec<String> },
}

/// 单个可调参数的定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamDef {
    /// 参数键名（唯一标识，英文 snake_case）
    pub key: String,
    /// 显示名称
    pub name: String,
    /// 参数说明
    pub description: String,
    /// 参数类型及约束
    pub param_type: ParamType,
    /// 默认值（JSON 值）
    pub default: serde_json::Value,
}

/// 单个子步骤的元数据
#[derive(Debug, Clone)]
pub struct StepMeta {
    /// 子步骤名称
    pub name: String,
    /// 子步骤描述
    pub description: String,
    /// 算法文档链接（可选）
    pub doc_url: Option<String>,
}

/// 一个 Phase（阶段）算法模块的完整元数据
#[derive(Debug, Clone)]
pub struct PhaseMeta {
    /// 唯一标识符（英文，如 "biome_division"）
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 阶段描述
    pub description: String,
    /// 子步骤列表（有序）
    pub steps: Vec<StepMeta>,
    /// 可调参数定义列表
    pub params: Vec<ParamDef>,
}

// ═══════════════════════════════════════════════════════════
// 运行时上下文 —— 引擎传给算法的执行环境
// ═══════════════════════════════════════════════════════════

/// 算法执行上下文
///
/// 包含算法执行一个子步骤所需的全部引擎资源。
/// 算法不应持有这些资源的所有权。
pub struct RuntimeContext<'a> {
    /// 世界方块数据（可读写）
    pub world: &'a mut World,
    /// 世界参数（尺寸、层级等，只读）
    pub profile: &'a WorldProfile,
    /// 方块定义表（只读）
    pub blocks: &'a [BlockDefinition],
    /// 环境定义表（只读）
    pub biomes: &'a [BiomeDefinition],
    /// 每步独立的确定性 RNG
    pub rng: &'a mut StdRng,
    /// 环境地图（共享状态，可读写）
    pub biome_map: &'a mut Option<BiomeMap>,
    /// 通用共享状态容器
    ///
    /// 算法可在此存放跨步骤/跨阶段的中间数据（如高度图、洞穴标记等）。
    /// 使用 `insert`/`get`/`get_mut` 并用 `downcast_ref`/`downcast_mut` 转换类型。
    ///
    /// # 示例
    /// ```ignore
    /// // 写入
    /// ctx.shared.insert("heightmap".into(), Box::new(vec![0u32; w * h]));
    /// // 读取
    /// let hm = ctx.shared.get("heightmap")
    ///     .and_then(|v| v.downcast_ref::<Vec<u32>>());
    /// ```
    pub shared: &'a mut HashMap<String, Box<dyn Any>>,
}

// ═══════════════════════════════════════════════════════════
// 核心 Trait —— 每个算法模块必须实现
// ═══════════════════════════════════════════════════════════

/// 阶段算法 trait
///
/// 每个算法模块实现此 trait，代表一个完整的生成阶段（Phase）。
/// 引擎通过此 trait 获取元数据、执行子步骤、读写参数。
///
/// # 实现示例
///
/// ```ignore
/// struct MyAlgorithm { /* 参数字段 */ }
///
/// impl PhaseAlgorithm for MyAlgorithm {
///     fn meta(&self) -> PhaseMeta {
///         PhaseMeta {
///             id: "my_algo".to_string(),
///             name: "我的算法".to_string(),
///             description: "示例算法".to_string(),
///             steps: vec![
///                 StepMeta { name: "步骤1".into(), description: "...".into(), doc_url: None },
///             ],
///             params: vec![],
///         }
///     }
///
///     fn execute(&self, step_index: usize, ctx: &mut RuntimeContext) -> Result<(), String> {
///         match step_index {
///             0 => { /* 步骤1的逻辑 */ Ok(()) }
///             _ => Err("无效步骤".into())
///         }
///     }
/// }
/// ```
pub trait PhaseAlgorithm {
    /// 返回此算法模块的完整元数据
    fn meta(&self) -> PhaseMeta;

    /// 执行指定子步骤
    ///
    /// `step_index`：子步骤索引（从 0 开始，对应 `meta().steps` 的下标）
    fn execute(&mut self, step_index: usize, ctx: &mut RuntimeContext) -> Result<(), String>;

    /// 返回当前参数值（用于持久化）
    ///
    /// 默认实现返回空对象 `{}`
    fn get_params(&self) -> serde_json::Value {
        serde_json::Value::Object(serde_json::Map::new())
    }

    /// 从 JSON 值恢复参数（从持久化加载）
    ///
    /// 默认实现忽略输入
    fn set_params(&mut self, _params: &serde_json::Value) {
        // 默认忽略
    }

    /// 管线重置时调用，清理算法内部运行时状态
    ///
    /// 默认实现什么都不做。如果算法有步骤间传递的内部状态，应在此清理。
    fn on_reset(&mut self) {
        // 默认忽略
    }
}
