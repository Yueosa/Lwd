use rand::Rng;

use crate::core::biome::{BiomeDefinition, BiomeId, BiomeMap};

use super::step::{GenerationContext, GenerationStep};

/// 环境划分步骤 — 水平方向划分世界环境
///
/// **简单算法**（v1）：
/// 1. 左右两侧固定海洋（各占 5-10%）
/// 2. 中间区域随机划分：根据 biome.json 中的 generation_weight 权重分配
/// 3. 每种环境宽度：300~800 列
pub struct BiomeDivisionStep {
    /// 环境定义列表
    biome_definitions: Vec<BiomeDefinition>,
    /// 海洋环境的ID（generation_weight = 0）
    ocean_id: Option<BiomeId>,
    /// 可生成环境的ID列表（generation_weight > 0）
    generatable_ids: Vec<BiomeId>,
    /// 累积权重（用于加权随机）
    cumulative_weights: Vec<u32>,
}

impl BiomeDivisionStep {
    pub fn new(biome_definitions: Vec<BiomeDefinition>) -> Self {
        // 找出海洋ID（权重为0）
        let ocean_id = biome_definitions
            .iter()
            .find(|b| b.generation_weight == 0)
            .map(|b| b.id);

        // 找出可生成的环境（权重>0）
        let generatable_ids: Vec<BiomeId> = biome_definitions
            .iter()
            .filter(|b| b.generation_weight > 0)
            .map(|b| b.id)
            .collect();

        // 计算累积权重
        let mut cumulative_weights = Vec::new();
        let mut sum = 0u32;
        for id in &generatable_ids {
            if let Some(biome) = biome_definitions.iter().find(|b| b.id == *id) {
                sum += biome.generation_weight;
                cumulative_weights.push(sum);
            }
        }

        Self {
            biome_definitions,
            ocean_id,
            generatable_ids,
            cumulative_weights,
        }
    }

    /// 根据权重随机选择一个环境ID
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

impl GenerationStep for BiomeDivisionStep {
    fn name(&self) -> &str {
        "环境划分"
    }

    fn description(&self) -> &str {
        "水平方向划分世界环境（海洋/森林/沙漠/雪地/丛林/猩红）"
    }

    fn execute(&self, ctx: &mut GenerationContext) -> Result<(), String> {
        let width = ctx.world.width;
        let mut biome_map = BiomeMap::new(width);

        // 如果没有海洋ID，返回错误
        let ocean_id = self
            .ocean_id
            .ok_or("biome.json 中未找到海洋环境（generation_weight=0）")?;

        // ── 1. 左侧海洋 ──
        let ocean_left_width = ctx.rng.gen_range(
            (width as f32 * 0.05) as u32..(width as f32 * 0.10) as u32,
        );
        biome_map.add_region(ocean_id, 0, ocean_left_width);

        // ── 2. 中间区域随机划分 ──
        let mut x = ocean_left_width;
        let ocean_right_start = width - ocean_left_width;

        while x < ocean_right_start {
            let biome_id = self.random_biome(ctx.rng);
            let min_width = 300u32.min(ocean_right_start - x);
            let max_width = 800u32.min(ocean_right_start - x);
            let biome_width = if min_width >= max_width {
                min_width
            } else {
                ctx.rng.gen_range(min_width..=max_width)
            };

            let end_x = (x + biome_width).min(ocean_right_start);
            biome_map.add_region(biome_id, x, end_x);
            x = end_x;
        }

        // ── 3. 右侧海洋 ──
        biome_map.add_region(ocean_id, ocean_right_start, width);

        // ── 4. 存入共享状态 ──
        ctx.state.biome_map = Some(biome_map);

        Ok(())
    }
}
