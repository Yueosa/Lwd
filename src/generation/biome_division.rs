use rand::Rng;

use crate::core::biome::{BiomeDefinition, BiomeId, BiomeMap};

use super::step::{GenerationContext, GenerationStep};

/// 共享的环境划分工具
struct BiomeWeights {
    /// 海洋环境的ID（generation_weight = 0）
    ocean_id: Option<BiomeId>,
    /// 可生成环境的ID列表（generation_weight > 0）
    generatable_ids: Vec<BiomeId>,
    /// 累积权重（用于加权随机）
    cumulative_weights: Vec<u32>,
}

impl BiomeWeights {
    fn from_definitions(biome_definitions: &[BiomeDefinition]) -> Self {
        let ocean_id = biome_definitions
            .iter()
            .find(|b| b.generation_weight == 0)
            .map(|b| b.id);

        let generatable_ids: Vec<BiomeId> = biome_definitions
            .iter()
            .filter(|b| b.generation_weight > 0)
            .map(|b| b.id)
            .collect();

        let mut cumulative_weights = Vec::new();
        let mut sum = 0u32;
        for id in &generatable_ids {
            if let Some(biome) = biome_definitions.iter().find(|b| b.id == *id) {
                sum += biome.generation_weight;
                cumulative_weights.push(sum);
            }
        }

        Self {
            ocean_id,
            generatable_ids,
            cumulative_weights,
        }
    }

    fn random_biome(&self, rng: &mut impl Rng) -> BiomeId {
        if self.generatable_ids.is_empty() || self.cumulative_weights.is_empty() {
            return 1; // fallback
        }
        let total = *self.cumulative_weights.last().unwrap();
        let roll = rng.gen_range(0..total);
        for (i, &cumulative) in self.cumulative_weights.iter().enumerate() {
            if roll < cumulative {
                return self.generatable_ids[i];
            }
        }
        self.generatable_ids[0]
    }
}

// ═══════════════════════════════════════════════════════════
// 1.0 海洋边界
// ═══════════════════════════════════════════════════════════

/// 子步骤 1.0 — 在世界两侧放置海洋环境
pub struct BiomeOceanBorderStep {
    weights: BiomeWeights,
}

impl BiomeOceanBorderStep {
    pub fn new(biome_definitions: Vec<BiomeDefinition>) -> Self {
        Self {
            weights: BiomeWeights::from_definitions(&biome_definitions),
        }
    }
}

impl GenerationStep for BiomeOceanBorderStep {
    fn execute(&self, ctx: &mut GenerationContext) -> Result<(), String> {
        let width = ctx.world.width;
        let ocean_id = self
            .weights
            .ocean_id
            .ok_or("biome.json 中未找到海洋环境（generation_weight=0）")?;

        let mut biome_map = BiomeMap::new(width);

        // 左侧海洋 (5~10%)
        let ocean_left_width = ctx.rng.gen_range(
            (width as f32 * 0.05) as u32..(width as f32 * 0.10) as u32,
        );
        biome_map.add_region(ocean_id, 0, ocean_left_width);

        // 右侧海洋（对称）
        let ocean_right_start = width - ocean_left_width;
        biome_map.add_region(ocean_id, ocean_right_start, width);

        ctx.state.biome_map = Some(biome_map);
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════
// 1.1 内陆划分
// ═══════════════════════════════════════════════════════════

/// 子步骤 1.1 — 根据权重随机分配各环境区域
pub struct BiomeInlandDivisionStep {
    weights: BiomeWeights,
}

impl BiomeInlandDivisionStep {
    pub fn new(biome_definitions: Vec<BiomeDefinition>) -> Self {
        Self {
            weights: BiomeWeights::from_definitions(&biome_definitions),
        }
    }
}

impl GenerationStep for BiomeInlandDivisionStep {
    fn execute(&self, ctx: &mut GenerationContext) -> Result<(), String> {
        let biome_map = ctx
            .state
            .biome_map
            .as_mut()
            .ok_or("内陆划分需要先执行海洋边界步骤 (1.0)")?;

        // 找到海洋边界之间的区域
        let regions = biome_map.regions();
        if regions.len() < 2 {
            return Err("海洋边界数据异常，需要至少2个区域".to_string());
        }

        // 左侧海洋的右端 = 内陆起点
        let inland_start = regions[0].end_x;
        // 右侧海洋的左端 = 内陆终点
        let inland_end = regions.last().unwrap().start_x;

        // 中间区域随机划分
        let mut x = inland_start;
        while x < inland_end {
            let biome_id = self.weights.random_biome(ctx.rng);
            let min_width = 300u32.min(inland_end - x);
            let max_width = 800u32.min(inland_end - x);
            let biome_width = if min_width >= max_width {
                min_width
            } else {
                ctx.rng.gen_range(min_width..=max_width)
            };

            let end_x = (x + biome_width).min(inland_end);
            biome_map.add_region(biome_id, x, end_x);
            x = end_x;
        }

        Ok(())
    }
}
