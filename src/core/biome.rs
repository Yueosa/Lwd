use crate::config::biome::BiomesConfig;
use crate::core::layer::LayerDefinition;

// ── Biome ID 和定义 ────────────────────────────────────────

pub type BiomeId = u8;

#[derive(Debug, Clone)]
pub struct BiomeDefinition {
    pub id: BiomeId,
    pub key: String,
    pub name: String,
    pub overlay_color: [u8; 4],
    pub generation_weight: u32,
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
            generation_weight: biome.generation_weight,
            description: biome.description.clone(),
        })
        .collect()
}

// ── 水平环境区域 ────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct BiomeRegion {
    pub biome_id: BiomeId,
    pub start_x: u32,
    pub end_x: u32,
}

impl BiomeRegion {
    pub fn width(&self) -> u32 {
        self.end_x - self.start_x
    }

    pub fn contains(&self, x: u32) -> bool {
        x >= self.start_x && x < self.end_x
    }
}

// ── 环境地图 ────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct BiomeMap {
    regions: Vec<BiomeRegion>,
    width: u32,
}

impl BiomeMap {
    pub fn new(width: u32) -> Self {
        Self {
            regions: Vec::new(),
            width,
        }
    }

    /// 添加环境区域
    pub fn add_region(&mut self, biome_id: BiomeId, start_x: u32, end_x: u32) {
        self.regions.push(BiomeRegion {
            biome_id,
            start_x,
            end_x,
        });
    }

    /// 获取某个 x 坐标的水平环境ID
    pub fn get_biome_at(&self, x: u32) -> Option<BiomeId> {
        self.regions
            .iter()
            .find(|r| r.contains(x))
            .map(|r| r.biome_id)
    }

    /// 获取所有区域（用于可视化）
    pub fn regions(&self) -> &[BiomeRegion] {
        &self.regions
    }

    /// 获取某个环境类型的所有区域
    pub fn get_regions_of_type(&self, biome_id: BiomeId) -> Vec<&BiomeRegion> {
        self.regions
            .iter()
            .filter(|r| r.biome_id == biome_id)
            .collect()
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
    /// 需要传入 biome_definitions 来查找名称
    pub fn label(&self, biome_definitions: &[BiomeDefinition]) -> String {
        let h = self
            .horizontal
            .and_then(|id| biome_definitions.iter().find(|b| b.id == id))
            .map(|b| b.name.as_str())
            .unwrap_or("未知环境");
        let v = self.vertical.as_deref().unwrap_or("未知层级");
        format!("{}+{}", h, v)
    }

    /// 简短标签（用于状态栏）
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

/// 层级名称缩写（用于简短显示）
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

// ── 通用环境检测函数 ───────────────────────────────────

/// **核心函数**：获取 (x, y) 处的完整环境信息
///
/// 结合水平环境地图和垂直层级配置，返回组合上下文。
///
/// # 示例
/// ```
/// let ctx = get_biome_context(1000, 400, &biome_map, &profile.layers, world.height);
/// println!("{}", ctx.label());  // "森林+地下"
/// ```
pub fn get_biome_context(
    x: u32,
    y: u32,
    biome_map: &BiomeMap,
    layers: &[LayerDefinition],
    world_height: u32,
) -> BiomeContext {
    BiomeContext {
        horizontal: biome_map.get_biome_at(x),
        vertical: get_layer_at(y, layers, world_height),
    }
}

/// 获取 y 坐标所属的层级名称
fn get_layer_at(y: u32, layers: &[LayerDefinition], world_height: u32) -> Option<String> {
    for layer in layers {
        let (start_row, end_row) = layer.bounds_for_height(world_height);
        if y >= start_row && y < end_row {
            return Some(layer.key.clone());
        }
    }
    None
}
