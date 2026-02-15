use crate::core::world::AIR_BLOCK_ID;

use super::step::{GenerationContext, GenerationStep};

/// Step 0 — fill the entire world with air.
pub struct ResetStep;

impl GenerationStep for ResetStep {
    fn name(&self) -> &str {
        "重置"
    }

    fn description(&self) -> &str {
        "将世界初始化为全空气"
    }

    fn execute(&self, ctx: &mut GenerationContext) -> Result<(), String> {
        ctx.world.tiles.fill(AIR_BLOCK_ID);
        Ok(())
    }
}
