use rayon::prelude::*;

use crate::config::world::{WorldConfig, WorldSize};
use crate::core::layer::{build_layers, LayerDefinition};
use crate::core::CoreError;

pub const AIR_BLOCK_ID: u8 = 1;

#[derive(Debug, Clone)]
pub struct WorldSizeSpec {
    pub key: String,
    pub width: u32,
    pub height: u32,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct WorldProfile {
    pub size: WorldSizeSpec,
    pub layers: Vec<LayerDefinition>,
}

#[derive(Debug, Clone)]
pub struct World {
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<u8>,
}

impl World {
    pub fn new_filled(width: u32, height: u32, block_id: u8) -> Self {
        let len = (width as usize) * (height as usize);
        Self {
            width,
            height,
            tiles: vec![block_id; len],
        }
    }

    pub fn new_air(width: u32, height: u32) -> Self {
        Self::new_filled(width, height, AIR_BLOCK_ID)
    }

    // ── 安全访问接口 ──────────────────────────────────────

    /// 获取 (x, y) 处的方块 ID，越界返回 None
    #[inline]
    pub fn get(&self, x: u32, y: u32) -> Option<u8> {
        if x < self.width && y < self.height {
            Some(self.tiles[(y * self.width + x) as usize])
        } else {
            None
        }
    }

    /// 获取 (x, y) 处的方块 ID，越界返回 AIR_BLOCK_ID
    #[inline]
    pub fn get_or_air(&self, x: u32, y: u32) -> u8 {
        self.get(x, y).unwrap_or(AIR_BLOCK_ID)
    }

    /// 设置 (x, y) 处的方块 ID，越界忽略
    #[inline]
    pub fn set(&mut self, x: u32, y: u32, block_id: u8) {
        if x < self.width && y < self.height {
            self.tiles[(y * self.width + x) as usize] = block_id;
        }
    }

    /// 填充矩形区域 [x0, x1) × [y0, y1)
    ///
    /// 大区域自动使用 rayon 并行填充。
    pub fn fill_rect(&mut self, x0: u32, y0: u32, x1: u32, y1: u32, block_id: u8) {
        let xs = x0.min(self.width) as usize;
        let xe = x1.min(self.width) as usize;
        let ys = y0.min(self.height) as usize;
        let ye = y1.min(self.height) as usize;
        let w = self.width as usize;

        let area = (xe - xs) * (ye - ys);
        if area >= 50_000 {
            // 并行填充
            self.tiles[ys * w..ye * w]
                .par_chunks_mut(w)
                .for_each(|row| {
                    for x in xs..xe {
                        row[x] = block_id;
                    }
                });
        } else {
            for y in ys..ye {
                let row_start = y * w;
                for x in xs..xe {
                    self.tiles[row_start + x] = block_id;
                }
            }
        }
    }

    /// 填充垂直一列 [y_start, y_end)
    pub fn fill_column(&mut self, x: u32, y_start: u32, y_end: u32, block_id: u8) {
        if x >= self.width {
            return;
        }
        let ys = y_start.min(self.height);
        let ye = y_end.min(self.height);
        for y in ys..ye {
            self.tiles[(y * self.width + x) as usize] = block_id;
        }
    }

    /// 遍历指定层级行范围 [y_start, y_end) 中的每个格子，调用闭包
    pub fn for_each_in_rows<F>(&mut self, y_start: u32, y_end: u32, mut f: F)
    where
        F: FnMut(u32, u32, &mut u8), // (x, y, tile)
    {
        let ys = y_start.min(self.height);
        let ye = y_end.min(self.height);
        for y in ys..ye {
            for x in 0..self.width {
                let idx = (y * self.width + x) as usize;
                f(x, y, &mut self.tiles[idx]);
            }
        }
    }

    /// 判断坐标是否在世界范围内
    #[inline]
    pub fn in_bounds(&self, x: u32, y: u32) -> bool {
        x < self.width && y < self.height
    }
}

impl WorldProfile {
    pub fn from_config(
        config: &WorldConfig,
        size_key: &str,
        custom_size: Option<(u32, u32)>,
    ) -> Result<Self, CoreError> {
        let size_cfg = config
            .world_sizes
            .get(size_key)
            .ok_or_else(|| CoreError::MissingWorldSize(size_key.to_string()))?;

        let (width, height) = resolve_size(size_key, size_cfg, custom_size)?;
        let size = WorldSizeSpec {
            key: size_key.to_string(),
            width,
            height,
            description: size_cfg.description.clone(),
        };

        let layers = build_layers(config)?;

        Ok(Self { size, layers })
    }

    pub fn create_world(&self) -> World {
        World::new_air(self.size.width, self.size.height)
    }
}

fn resolve_size(
    size_key: &str,
    size_cfg: &WorldSize,
    custom_size: Option<(u32, u32)>,
) -> Result<(u32, u32), CoreError> {
    if size_key == "custom" {
        if let Some((width, height)) = custom_size {
            if width > 0 && height > 0 {
                return Ok((width, height));
            }
        }
        return Err(CoreError::InvalidCustomSize);
    }

    match (size_cfg.width, size_cfg.height) {
        (Some(width), Some(height)) if width > 0 && height > 0 => Ok((width, height)),
        _ => Err(CoreError::InvalidCustomSize),
    }
}
