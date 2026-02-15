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

    /// 将一个矩形区域全部设为指定 biome
    pub fn fill_rect(&mut self, x0: u32, y0: u32, x1: u32, y1: u32, biome: BiomeId) {
        let x_start = x0.min(self.width);
        let x_end = x1.min(self.width);
        let y_start = y0.min(self.height);
        let y_end = y1.min(self.height);
        for y in y_start..y_end {
            for x in x_start..x_end {
                self.data[(y * self.width + x) as usize] = biome;
            }
        }
    }

    /// 将一个垂直列范围设为指定 biome
    pub fn fill_column(&mut self, x: u32, y_start: u32, y_end: u32, biome: BiomeId) {
        if x >= self.width {
            return;
        }
        let ys = y_start.min(self.height);
        let ye = y_end.min(self.height);
        for y in ys..ye {
            self.data[(y * self.width + x) as usize] = biome;
        }
    }

    /// 填充椭圆形区域
    /// center: (cx, cy), radii: (rx, ry)
    pub fn fill_ellipse(&mut self, cx: f64, cy: f64, rx: f64, ry: f64, biome: BiomeId) {
        let x0 = ((cx - rx).floor().max(0.0)) as u32;
        let x1 = ((cx + rx).ceil().min(self.width as f64)) as u32;
        let y0 = ((cy - ry).floor().max(0.0)) as u32;
        let y1 = ((cy + ry).ceil().min(self.height as f64)) as u32;

        for y in y0..y1 {
            for x in x0..x1 {
                let dx = (x as f64 - cx) / rx;
                let dy = (y as f64 - cy) / ry;
                if dx * dx + dy * dy <= 1.0 {
                    self.data[(y * self.width + x) as usize] = biome;
                }
            }
        }
    }

    /// 填充梯形区域（左右边界随 y 线性变化）
    ///
    /// top_x0..top_x1 是顶边宽度范围 (y = y_top)
    /// bot_x0..bot_x1 是底边宽度范围 (y = y_bot)
    pub fn fill_trapezoid(
        &mut self,
        y_top: u32,
        y_bot: u32,
        top_x0: f64,
        top_x1: f64,
        bot_x0: f64,
        bot_x1: f64,
        biome: BiomeId,
    ) {
        if y_top >= y_bot {
            return;
        }
        let h = (y_bot - y_top) as f64;
        let yt = y_top.min(self.height);
        let yb = y_bot.min(self.height);
        for y in yt..yb {
            let t = (y - y_top) as f64 / h;
            let left = top_x0 + (bot_x0 - top_x0) * t;
            let right = top_x1 + (bot_x1 - top_x1) * t;
            let xl = (left.floor().max(0.0)) as u32;
            let xr = (right.ceil().min(self.width as f64)) as u32;
            for x in xl..xr {
                self.data[(y * self.width + x) as usize] = biome;
            }
        }
    }

    /// 仅在当前为 `only_if` 的格子上填充椭圆
    pub fn fill_ellipse_if(
        &mut self,
        cx: f64,
        cy: f64,
        rx: f64,
        ry: f64,
        biome: BiomeId,
        only_if: BiomeId,
    ) {
        let x0 = ((cx - rx).floor().max(0.0)) as u32;
        let x1 = ((cx + rx).ceil().min(self.width as f64)) as u32;
        let y0 = ((cy - ry).floor().max(0.0)) as u32;
        let y1 = ((cy + ry).ceil().min(self.height as f64)) as u32;

        for y in y0..y1 {
            for x in x0..x1 {
                let idx = (y * self.width + x) as usize;
                if self.data[idx] != only_if {
                    continue;
                }
                let dx = (x as f64 - cx) / rx;
                let dy = (y as f64 - cy) / ry;
                if dx * dx + dy * dy <= 1.0 {
                    self.data[idx] = biome;
                }
            }
        }
    }

    /// 返回底层数据的只读引用（用于渲染）
    pub fn data(&self) -> &[BiomeId] {
        &self.data
    }

    /// 统计指定 biome 在某个 x 范围内的格子数（用于判定密度）
    pub fn count_biome_in_x_range(&self, biome: BiomeId, x_start: u32, x_end: u32) -> usize {
        let xs = x_start.min(self.width);
        let xe = x_end.min(self.width);
        let mut count = 0usize;
        for y in 0..self.height {
            for x in xs..xe {
                if self.data[(y * self.width + x) as usize] == biome {
                    count += 1;
                }
            }
        }
        count
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
