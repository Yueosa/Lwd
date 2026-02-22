use rayon::prelude::*;

use crate::config::biome::BiomesConfig;
use crate::core::layer::LayerDefinition;

// ── Biome ID 和定义 ────────────────────────────────────────

pub type BiomeId = u8;

/// 特殊 Biome ID：未分配 (0 表示尚未被任何步骤处理)
pub const BIOME_UNASSIGNED: BiomeId = 0;

#[derive(Debug, Clone)]
pub struct BiomeDefinition {
    pub id: BiomeId,
    pub key: String,
    pub name: String,
    pub overlay_color: [u8; 4],
    pub description: String,
}

pub fn build_biome_definitions(config: &BiomesConfig) -> Vec<BiomeDefinition> {
    config
        .iter()
        .map(|(id, biome)| BiomeDefinition {
            id: *id,
            key: biome.key.clone(),
            name: biome.name.clone(),
            overlay_color: biome.overlay_color,
            description: biome.description.clone(),
        })
        .collect()
}

// ── 2D 环境地图 ────────────────────────────────────────

/// 二维环境地图：每个格子都有一个 BiomeId。
///
/// 支持有形状的环境区域（梯形、椭圆等），而不仅仅是水平条带。
#[derive(Debug, Clone)]
pub struct BiomeMap {
    pub width: u32,
    pub height: u32,
    /// 行优先存储: data[y * width + x]
    data: Vec<BiomeId>,
}

impl BiomeMap {
    /// 创建一个全部填充为指定 biome 的地图
    pub fn new_filled(width: u32, height: u32, fill: BiomeId) -> Self {
        let len = (width as usize) * (height as usize);
        Self {
            width,
            height,
            data: vec![fill; len],
        }
    }

    /// 获取 (x, y) 处的 biome
    pub fn get(&self, x: u32, y: u32) -> BiomeId {
        if x >= self.width || y >= self.height {
            return BIOME_UNASSIGNED;
        }
        self.data[(y * self.width + x) as usize]
    }

    /// 设置 (x, y) 处的 biome
    pub fn set(&mut self, x: u32, y: u32, biome: BiomeId) {
        if x < self.width && y < self.height {
            self.data[(y * self.width + x) as usize] = biome;
        }
    }

    /// 返回底层数据的只读引用（用于渲染）
    pub fn data(&self) -> &[BiomeId] {
        &self.data
    }

    /// 返回底层数据的可变引用（用于并行写入）
    pub fn data_mut(&mut self) -> &mut [BiomeId] {
        &mut self.data
    }

    /// 统计指定 biome 在某个 x 范围内的格子数（用于判定密度）
    ///
    /// 使用 rayon 并行按行统计。
    pub fn count_biome_in_x_range(&self, biome: BiomeId, x_start: u32, x_end: u32) -> usize {
        let xs = x_start.min(self.width) as usize;
        let xe = x_end.min(self.width) as usize;
        let w = self.width as usize;

        self.data
            .par_chunks(w)
            .map(|row| {
                row[xs..xe].iter().filter(|&&b| b == biome).count()
            })
            .sum()
    }
}

// ── 环境上下文（组合信息）──────────────────────────────

/// 某个坐标点的完整环境信息
#[derive(Debug, Clone)]
pub struct BiomeContext {
    /// 水平环境ID
    pub horizontal: Option<BiomeId>,
    /// 垂直层级名称（"surface"/"underground"/"cavern"）
    pub vertical: Option<String>,
}

impl BiomeContext {
    /// 生成可读的组合标签，如 "森林+地表" 或 "海洋+洞穴"
    pub fn label(&self, biome_definitions: &[BiomeDefinition]) -> String {
        let h = self
            .horizontal
            .and_then(|id| biome_definitions.iter().find(|b| b.id == id))
            .map(|b| b.name.as_str())
            .unwrap_or("未知环境");
        let v = self.vertical.as_deref().unwrap_or("未知层级");
        format!("{}+{}", h, v)
    }

    /// 简短标签
    pub fn short_label(&self, biome_definitions: &[BiomeDefinition]) -> String {
        if let Some(id) = self.horizontal {
            if let Some(biome) = biome_definitions.iter().find(|b| b.id == id) {
                if let Some(v) = &self.vertical {
                    return format!("{}·{}", biome.name, layer_short_name(v));
                }
            }
        }
        "未知".to_string()
    }
}

fn layer_short_name(key: &str) -> &'static str {
    match key {
        "space" => "太空",
        "surface" => "地表",
        "underground" => "地下",
        "cavern" => "洞穴",
        "hell" => "地狱",
        _ => "?",
    }
}

/// 获取 (x, y) 处的完整环境信息
pub fn get_biome_context(
    x: u32,
    y: u32,
    biome_map: &BiomeMap,
    layers: &[LayerDefinition],
    world_height: u32,
) -> BiomeContext {
    BiomeContext {
        horizontal: Some(biome_map.get(x, y)),
        vertical: get_layer_at(y, layers, world_height),
    }
}

fn get_layer_at(y: u32, layers: &[LayerDefinition], world_height: u32) -> Option<String> {
    for layer in layers {
        let (start_row, end_row) = layer.bounds_for_height(world_height);
        if y >= start_row && y < end_row {
            return Some(layer.key.clone());
        }
    }
    None
}
