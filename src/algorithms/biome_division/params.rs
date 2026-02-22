//! 环境判定算法参数定义

use serde::{Deserialize, Serialize};
use crate::core::layer::LayerDefinition;

/// 辅助函数：从层级列表中查找指定 key 的起始百分比（0.0~1.0）
fn layer_start(layers: &[LayerDefinition], key: &str) -> f64 {
    layers.iter()
        .find(|l| l.key == key)
        .map(|l| l.start_percent as f64 / 100.0)
        .unwrap_or(0.0)
}

/// 辅助函数：从层级列表中查找指定 key 的结束百分比（0.0~1.0）
fn layer_end(layers: &[LayerDefinition], key: &str) -> f64 {
    layers.iter()
        .find(|l| l.key == key)
        .map(|l| l.end_percent as f64 / 100.0)
        .unwrap_or(1.0)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeDivisionParams {
    // 海洋生成
    pub ocean_left_width: f64,
    pub ocean_right_width: f64,
    pub ocean_top_limit: f64,
    pub ocean_bottom_limit: f64,
    
    // 森林生成
    pub forest_width_ratio: f64,
    
    // 丛林生成
    pub jungle_width_ratio: f64,
    pub jungle_top_limit: f64,
    pub jungle_bottom_limit: f64,
    pub jungle_center_offset_range: f64,
    
    // 雪原生成
    pub snow_top_width_ratio: f64,
    pub snow_bottom_width_ratio: f64,
    pub snow_top_limit: f64,
    pub snow_bottom_limit: f64,
    pub snow_bottom_depth_factor: f64,
    pub snow_center_offset_range: f64,
    
    // 沙漠生成
    pub desert_surface_count: u32,
    pub desert_surface_width_min: f64,
    pub desert_surface_width_max: f64,
    pub desert_surface_top_limit: f64,
    pub desert_surface_bottom_limit: f64,
    pub desert_surface_min_spacing: f64,
    pub desert_true_count: u32,
    pub desert_true_top_limit: f64,
    pub desert_true_bottom_limit: f64,
    pub desert_true_depth_factor: f64,
    
    // 猩红生成
    pub crimson_count: u32,
    pub crimson_width_min: f64,
    pub crimson_width_max: f64,
    pub crimson_top_limit: f64,
    pub crimson_bottom_limit: f64,
    pub crimson_min_spacing: f64,
    
    // 森林填充
    pub forest_fill_merge_threshold: u32,
}

impl BiomeDivisionParams {
    /// 从层级配置构建参数默认值（所有层级边界从配置读取，不硬编码）
    pub fn from_layers(layers: &[LayerDefinition]) -> Self {
        let surface_start = layer_start(layers, "surface");
        let underground_end = layer_end(layers, "underground");
        let underground_start = layer_start(layers, "underground");
        let cavern_end = layer_end(layers, "cavern");

        Self {
            ocean_left_width: 0.05,
            ocean_right_width: 0.05,
            ocean_top_limit: surface_start,
            ocean_bottom_limit: underground_end,
            forest_width_ratio: 0.05,
            jungle_width_ratio: 0.12,
            jungle_top_limit: surface_start,
            jungle_bottom_limit: cavern_end,
            jungle_center_offset_range: 0.20,
            snow_top_width_ratio: 0.08,
            snow_bottom_width_ratio: 0.20,
            snow_top_limit: surface_start,
            snow_bottom_limit: cavern_end,
            snow_bottom_depth_factor: 0.8,
            snow_center_offset_range: 0.12,
            desert_surface_count: 3,
            desert_surface_width_min: 0.03,
            desert_surface_width_max: 0.05,
            desert_surface_top_limit: surface_start,
            desert_surface_bottom_limit: underground_end,
            desert_surface_min_spacing: 0.15,
            desert_true_count: 1,
            desert_true_top_limit: underground_start,
            desert_true_bottom_limit: cavern_end,
            desert_true_depth_factor: 0.90,
            crimson_count: 3,
            crimson_width_min: 0.025,
            crimson_width_max: 0.1,
            crimson_top_limit: surface_start,
            crimson_bottom_limit: underground_end,
            crimson_min_spacing: 0.15,
            forest_fill_merge_threshold: 100,
        }
    }
}

impl Default for BiomeDivisionParams {
    fn default() -> Self {
        // 回退默认值（与 world.json 默认层级配置一致）
        Self {
            ocean_left_width: 0.05,
            ocean_right_width: 0.05,
            ocean_top_limit: 0.10,
            ocean_bottom_limit: 0.40,
            forest_width_ratio: 0.05,
            jungle_width_ratio: 0.12,
            jungle_top_limit: 0.10,
            jungle_bottom_limit: 0.85,
            jungle_center_offset_range: 0.20,
            snow_top_width_ratio: 0.08,
            snow_bottom_width_ratio: 0.20,
            snow_top_limit: 0.10,
            snow_bottom_limit: 0.85,
            snow_bottom_depth_factor: 0.8,
            snow_center_offset_range: 0.12,
            desert_surface_count: 3,
            desert_surface_width_min: 0.03,
            desert_surface_width_max: 0.05,
            desert_surface_top_limit: 0.10,
            desert_surface_bottom_limit: 0.40,
            desert_surface_min_spacing: 0.15,
            desert_true_count: 1,
            desert_true_top_limit: 0.30,
            desert_true_bottom_limit: 0.85,
            desert_true_depth_factor: 0.90,
            crimson_count: 3,
            crimson_width_min: 0.025,
            crimson_width_max: 0.1,
            crimson_top_limit: 0.10,
            crimson_bottom_limit: 0.40,
            crimson_min_spacing: 0.15,
            forest_fill_merge_threshold: 100,
        }
    }
}
