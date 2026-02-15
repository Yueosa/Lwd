//! # 世界快照模型
//!
//! `WorldSnapshot` 是能完整复现一个世界的最小信息集合。
//!
//! **设计原则**：不存方块数据，只存 seed + params + 配置。
//! 导入时 replay 整个 pipeline 即可还原（确定性 RNG 保证）。

use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// 存档格式当前版本
pub const SNAPSHOT_VERSION: u32 = 1;

/// 层级参数覆盖
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerOverride {
    pub start_percent: u8,
    pub end_percent: u8,
}

/// 单个算法模块的参数快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmState {
    /// 算法模块 ID（对应 `PhaseMeta.id`）
    pub algorithm_id: String,
    /// 参数值（`PhaseAlgorithm::get_params()` 的返回值）
    pub params: serde_json::Value,
}

/// 世界快照 — 完整复现一个世界所需的全部信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSnapshot {
    /// 存档格式版本
    pub version: u32,
    /// 主种子
    pub seed: u64,
    /// 世界尺寸键名 ("small" / "medium" / "large")
    pub world_size: String,
    /// 层级配置覆盖（key → LayerOverride）
    pub layers: HashMap<String, LayerOverride>,
    /// 各算法模块的参数快照（按注册顺序）
    pub algorithms: Vec<AlgorithmState>,
    /// 导出时的 Unix 时间戳（秒）
    pub timestamp: u64,
}

impl WorldSnapshot {
    /// 获取当前 Unix 时间戳
    fn now_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// 保存为 `.lwd` 文件（JSON 格式）
    pub fn save_lwd(&self, path: &Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("序列化失败: {e}"))?;
        std::fs::write(path, json)
            .map_err(|e| format!("写入文件失败: {e}"))?;
        Ok(())
    }

    /// 从 `.lwd` 文件加载
    pub fn load_lwd(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("读取文件失败: {e}"))?;
        let snapshot: WorldSnapshot = serde_json::from_str(&content)
            .map_err(|e| format!("解析存档失败: {e}"))?;
        if snapshot.version > SNAPSHOT_VERSION {
            return Err(format!(
                "存档版本 {} 高于当前支持的版本 {}",
                snapshot.version, SNAPSHOT_VERSION
            ));
        }
        Ok(snapshot)
    }

    /// 从当前运行状态收集快照
    pub fn collect(
        seed: u64,
        world_size: &str,
        layers: &[crate::core::layer::LayerDefinition],
        algorithms: &[Box<dyn crate::generation::algorithm::PhaseAlgorithm>],
    ) -> Self {
        let layer_overrides: HashMap<String, LayerOverride> = layers
            .iter()
            .map(|l| {
                (
                    l.key.clone(),
                    LayerOverride {
                        start_percent: l.start_percent,
                        end_percent: l.end_percent,
                    },
                )
            })
            .collect();

        let algo_states: Vec<AlgorithmState> = algorithms
            .iter()
            .map(|a| {
                let meta = a.meta();
                AlgorithmState {
                    algorithm_id: meta.id,
                    params: a.get_params(),
                }
            })
            .collect();

        Self {
            version: SNAPSHOT_VERSION,
            seed,
            world_size: world_size.to_string(),
            layers: layer_overrides,
            algorithms: algo_states,
            timestamp: Self::now_timestamp(),
        }
    }
}

/// 将世界方块数据导出为 PNG 文件
pub fn export_png(
    world: &crate::core::world::World,
    color_lut: &[egui::Color32; 256],
    path: &Path,
) -> Result<(), String> {
    let w = world.width;
    let h = world.height;

    let mut buf: Vec<u8> = Vec::with_capacity((w * h * 4) as usize);
    for &tile in &world.tiles {
        let c = color_lut[tile as usize];
        buf.push(c.r());
        buf.push(c.g());
        buf.push(c.b());
        buf.push(c.a());
    }

    let img = image::RgbaImage::from_raw(w, h, buf)
        .ok_or_else(|| "创建图像缓冲区失败".to_string())?;
    img.save(path)
        .map_err(|e| format!("保存 PNG 失败: {e}"))?;

    Ok(())
}
