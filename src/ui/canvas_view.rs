use egui::{Color32, ColorImage, Pos2, Rect, Sense, Stroke, TextureHandle, TextureOptions, Ui, Vec2};
use rayon::prelude::*;

use crate::core::biome::{BiomeDefinition, BiomeMap};
use crate::core::layer::LayerDefinition;
use crate::rendering::viewport::ViewportState;

#[derive(Debug, Clone, Copy)]
pub struct HoverInfo {
    pub x: u32,
    pub y: u32,
}

/// 从 2D BiomeMap 生成半透明 overlay 纹理（rayon 并行按行生成）
fn biome_overlay_image(
    biome_map: &BiomeMap,
    biome_definitions: &[BiomeDefinition],
) -> ColorImage {
    let w = biome_map.width as usize;
    let h = biome_map.height as usize;

    // 构建 biome ID → overlay 颜色 LUT（最多 256）
    let mut biome_lut = [Color32::TRANSPARENT; 256];
    for bdef in biome_definitions {
        let c = bdef.overlay_color;
        biome_lut[bdef.id as usize] = Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
    }

    let data = biome_map.data();
    let mut pixels = vec![Color32::TRANSPARENT; w * h];

    // 按行并行：每行独立做 LUT 查表
    pixels
        .par_chunks_mut(w)
        .enumerate()
        .for_each(|(y, row_pixels)| {
            let row_start = y * w;
            for x in 0..w {
                row_pixels[x] = biome_lut[data[row_start + x] as usize];
            }
        });

    ColorImage {
        size: [w, h],
        pixels,
    }
}

/// 在 biome overlay 上找到各区域的中心并标注名称。
///
/// 对于分布在世界两侧的环境（如海洋），会检测不连续区域并分别标注。
/// 每个区域独立计算 avg_y，避免跨区域平均导致文字错位。
/// 包含碰撞检测：如果标签重叠则尝试偏移，偏移后仍重叠则跳过小区域标签。
fn draw_biome_labels(
    painter: &egui::Painter,
    biome_map: &BiomeMap,
    biome_definitions: &[BiomeDefinition],
    image_rect: Rect,
    zoom: f32,
) {
    use std::collections::HashMap;

    // 对每种 biome 收集所有采样点 (x, y)，跳过 UNASSIGNED
    let mut sample_map: HashMap<u8, Vec<(u32, u32)>> = HashMap::new();

    let step = 8u32;
    let w = biome_map.width;
    let h = biome_map.height;
    let mut y = 0;
    while y < h {
        let mut x = 0;
        while x < w {
            let bid = biome_map.get(x, y);
            if bid != 0 {
                sample_map.entry(bid).or_default().push((x, y));
            }
            x += step;
        }
        y += step;
    }

    // 间隔阈值：如果连续采样点之间 x 间距超过此值，视为两个独立区域
    // 使用采样步长的 3 倍 (24px)。旧值 w/5 (20%世界宽度) 会将
    // 相距 15% 的小环境（如沙漠）错误合并成一个区域，导致标签错位
    let gap_threshold = step * 3;

    // 第一阶段：收集所有候选标签 (pos, text, region_size)
    struct LabelCandidate {
        pos: Pos2,
        text: String,
        region_size: u64,
    }
    let mut candidates: Vec<LabelCandidate> = Vec::new();

    for (bid, points) in &sample_map {
        let bdef = match biome_definitions.iter().find(|d| d.id == *bid) {
            Some(d) => d,
            None => continue,
        };

        // 按 x 排序
        let mut sorted_points = points.clone();
        sorted_points.sort_unstable_by_key(|&(x, _)| x);

        // 按 x 间隔切分成独立区域
        struct Region {
            x_min: u32,
            x_max: u32,
            sum_y: u64,
            count: u64,
        }
        let mut regions: Vec<Region> = Vec::new();
        let mut cur = Region {
            x_min: sorted_points[0].0,
            x_max: sorted_points[0].0,
            sum_y: sorted_points[0].1 as u64,
            count: 1,
        };

        for &(x, y) in &sorted_points[1..] {
            if x - cur.x_max > gap_threshold {
                regions.push(cur);
                cur = Region { x_min: x, x_max: x, sum_y: y as u64, count: 1 };
            } else {
                cur.x_max = x;
                cur.sum_y += y as u64;
                cur.count += 1;
            }
        }
        regions.push(cur);

        for region in &regions {
            let cx = ((region.x_min + region.x_max) as f32 / 2.0) * zoom + image_rect.left();
            let cy = (region.sum_y as f32 / region.count as f32) * zoom + image_rect.top();
            let pos = Pos2::new(cx, cy);
            if image_rect.contains(pos) {
                candidates.push(LabelCandidate {
                    pos,
                    text: bdef.name.clone(),
                    region_size: region.count,
                });
            }
        }
    }

    // 按区域大小降序排列（大区域优先放置）
    candidates.sort_by(|a, b| b.region_size.cmp(&a.region_size));

    // 第二阶段：逐个放置，碰撞检测
    let font = egui::FontId::proportional(14.0);
    let mut placed_rects: Vec<Rect> = Vec::new();
    let label_padding = 4.0_f32;

    for candidate in &candidates {
        // 估算文字包围盒
        let galley = painter.layout_no_wrap(candidate.text.clone(), font.clone(), Color32::WHITE);
        let text_size = galley.size();
        let half_w = text_size.x / 2.0 + label_padding;
        let half_h = text_size.y / 2.0 + label_padding;

        let make_rect = |pos: Pos2| -> Rect {
            Rect::from_min_max(
                Pos2::new(pos.x - half_w, pos.y - half_h),
                Pos2::new(pos.x + half_w, pos.y + half_h),
            )
        };

        let overlaps = |r: &Rect| -> bool {
            placed_rects.iter().any(|pr| pr.intersects(*r))
        };

        // 尝试多个偏移位置：原位 → 上/下/左/右 → 对角线
        let base = candidate.pos;
        let dy_step = text_size.y + 6.0;
        let dx_step = text_size.x * 0.6 + 6.0;
        let offsets: [(f32, f32); 7] = [
            (0.0,     0.0),      // 原位
            (0.0,     -dy_step), // 上移
            (0.0,      dy_step), // 下移
            (-dx_step, 0.0),     // 左移
            ( dx_step, 0.0),     // 右移
            (-dx_step, -dy_step),// 左上
            ( dx_step, -dy_step),// 右上
        ];
        let mut placed = false;
        for &(dx, dy) in &offsets {
            let pos = Pos2::new(base.x + dx, base.y + dy);
            if !image_rect.contains(pos) {
                continue;
            }
            let r = make_rect(pos);
            if !overlaps(&r) {
                painter.text(pos, egui::Align2::CENTER_CENTER, &candidate.text, font.clone(), Color32::WHITE);
                placed_rects.push(r);
                placed = true;
                break;
            }
        }

        // 如果所有位置都冲突，跳过此标签（小区域的标签被舍弃）
        let _ = placed;
    }
}

pub fn show_canvas(
    ui: &mut Ui,
    texture: &TextureHandle,
    world_width: u32,
    world_height: u32,
    viewport: &mut ViewportState,
    biome_map: Option<&BiomeMap>,
    biome_definitions: &[BiomeDefinition],
    layers: &[LayerDefinition],
    show_biome_color: bool,
    show_biome_labels: bool,
    show_layer_lines: bool,
    show_layer_labels: bool,
    biome_overlay_texture: &mut Option<TextureHandle>,
) -> Option<HoverInfo> {
    let available = ui.available_size();
    let (rect, response) = ui.allocate_exact_size(available, Sense::click_and_drag());

    // ── background checkerboard ──────────────────────────────
    let painter = ui.painter_at(rect);
    let tile = 48.0;
    let c0 = Color32::from_gray(28);
    let c1 = Color32::from_gray(35);
    let cols = (rect.width() / tile).ceil() as i32;
    let rows = (rect.height() / tile).ceil() as i32;
    for r in 0..rows {
        for c in 0..cols {
            let min = Pos2::new(rect.left() + c as f32 * tile, rect.top() + r as f32 * tile);
            let max = Pos2::new(
                (min.x + tile).min(rect.right()),
                (min.y + tile).min(rect.bottom()),
            );
            let color = if (r + c) % 2 == 0 { c0 } else { c1 };
            painter.rect_filled(Rect::from_min_max(min, max), 0.0, color);
        }
    }

    // ── world image ──────────────────────────────────────────
    let image_size = Vec2::new(
        world_width as f32 * viewport.zoom,
        world_height as f32 * viewport.zoom,
    );
    let center = rect.center() + Vec2::new(viewport.offset[0], viewport.offset[1]);
    let image_rect = Rect::from_center_size(center, image_size);

    painter.image(
        texture.id(),
        image_rect,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );
    painter.rect_stroke(image_rect, 0.0, Stroke::new(1.0, Color32::from_gray(120)));

    // ── biome overlay ──────────────────────────────────────────
    if show_biome_color || show_biome_labels {
        if let Some(bm) = biome_map {
            // 覆盖色纹理
            if show_biome_color {
                if biome_overlay_texture.is_none() {
                    let img = biome_overlay_image(bm, biome_definitions);
                    *biome_overlay_texture = Some(ui.ctx().load_texture(
                        "biome_overlay",
                        img,
                        TextureOptions::NEAREST,
                    ));
                }

                if let Some(ov_tex) = biome_overlay_texture {
                    painter.image(
                        ov_tex.id(),
                        image_rect,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
            }

            // 环境名称文字标签
            if show_biome_labels {
                draw_biome_labels(&painter, bm, biome_definitions, image_rect, viewport.zoom);
            }
        }
    }

    // ── layer overlay ──────────────────────────────────────────
    if show_layer_lines || show_layer_labels {
        if show_layer_lines {
            // 收集所有唯一的边界百分比值
            let mut boundary_percents: Vec<u8> = Vec::new();
            for layer in layers {
                if !boundary_percents.contains(&layer.start_percent) {
                    boundary_percents.push(layer.start_percent);
                }
                if !boundary_percents.contains(&layer.end_percent) {
                    boundary_percents.push(layer.end_percent);
                }
            }
            boundary_percents.sort();
            
            // 绘制所有边界线
            for &pct in &boundary_percents {
                let y_percent = pct as f32 / 100.0;
                let y_world = (world_height as f32 * y_percent) as u32;
                let y_screen = image_rect.top() + y_world as f32 * viewport.zoom;
                
                if y_screen >= image_rect.top() && y_screen <= image_rect.bottom() {
                    let start = Pos2::new(image_rect.left(), y_screen);
                    let end = Pos2::new(image_rect.right(), y_screen);
                    
                    painter.line_segment(
                        [start, end],
                        Stroke::new(1.5, Color32::from_rgba_unmultiplied(255, 255, 255, 120)),
                    );
                }
            }
        }
        
        // 在每个层级的中心位置绘制名称标签
        if show_layer_labels {
            for layer in layers {
                let mid_percent = (layer.start_percent as f32 + layer.end_percent as f32) / 2.0 / 100.0;
                let mid_y = image_rect.top() + world_height as f32 * mid_percent * viewport.zoom;
                
                if mid_y >= image_rect.top() && mid_y <= image_rect.bottom() {
                    let label_pos = Pos2::new(image_rect.left() + 10.0, mid_y);
                    painter.text(
                        label_pos,
                        egui::Align2::LEFT_CENTER,
                        &layer.key,
                        egui::FontId::proportional(12.0),
                        Color32::from_rgba_unmultiplied(255, 255, 255, 180),
                    );
                }
            }
        }
    }

    // ── minimap (bottom-right thumbnail) ────────────────────
    {
        use crate::ui::theme;

        let minimap_max_w: f32 = 180.0;
        let minimap_max_h: f32 = 110.0;
        let margin: f32 = 12.0;

        // 按世界宽高比缩放
        let world_aspect = world_width as f32 / world_height.max(1) as f32;
        let (mw, mh) = if world_aspect > minimap_max_w / minimap_max_h {
            (minimap_max_w, minimap_max_w / world_aspect)
        } else {
            (minimap_max_h * world_aspect, minimap_max_h)
        };

        let minimap_rect = Rect::from_min_size(
            Pos2::new(rect.right() - mw - margin, rect.bottom() - mh - margin),
            Vec2::new(mw, mh),
        );

        // 半透明背景
        painter.rect_filled(
            minimap_rect.expand(2.0),
            4.0,
            Color32::from_rgba_unmultiplied(30, 30, 40, 200),
        );

        // 绘制世界缩略图
        painter.image(
            texture.id(),
            minimap_rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::from_rgba_unmultiplied(255, 255, 255, 220),
        );

        // 计算当前可视区域在世界坐标中的范围
        let vis_left = ((rect.left() - image_rect.left()) / viewport.zoom).clamp(0.0, world_width as f32);
        let vis_top = ((rect.top() - image_rect.top()) / viewport.zoom).clamp(0.0, world_height as f32);
        let vis_right = ((rect.right() - image_rect.left()) / viewport.zoom).clamp(0.0, world_width as f32);
        let vis_bottom = ((rect.bottom() - image_rect.top()) / viewport.zoom).clamp(0.0, world_height as f32);

        // 映射到 minimap 坐标
        let scale_x = mw / world_width as f32;
        let scale_y = mh / world_height as f32;
        let vp_rect = Rect::from_min_max(
            Pos2::new(
                minimap_rect.left() + vis_left * scale_x,
                minimap_rect.top() + vis_top * scale_y,
            ),
            Pos2::new(
                minimap_rect.left() + vis_right * scale_x,
                minimap_rect.top() + vis_bottom * scale_y,
            ),
        );

        // 视口矩形指示器（蓝色边框 + 半透明填充）
        let vp_clipped = vp_rect.intersect(minimap_rect);
        if vp_clipped.width() > 0.0 && vp_clipped.height() > 0.0 {
            painter.rect_filled(
                vp_clipped,
                0.0,
                Color32::from_rgba_unmultiplied(91, 206, 250, 35),
            );
            painter.rect_stroke(
                vp_clipped,
                0.0,
                Stroke::new(1.5, theme::BLUE_LIGHT),
            );
        }

        // minimap 边框 
        painter.rect_stroke(
            minimap_rect,
            4.0,
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(91, 206, 250, 100)),
        );
    }

    // ── drag to pan ──────────────────────────────────────────
    if response.dragged() {
        let delta = response.drag_delta();
        viewport.pan(delta.x, delta.y);
    }

    // ── scroll wheel to zoom (anchored at cursor) ────────────
    let hovered = response.hovered() || response.dragged();
    if hovered {
        let scroll = ui.ctx().input(|i| i.smooth_scroll_delta);
        if scroll.y.abs() > 0.5 {
            if let Some(pointer) = ui.ctx().input(|i| i.pointer.hover_pos()) {
                let old_zoom = viewport.zoom;
                // Gentle: ~5% zoom per typical scroll tick (~50px)
                let factor = (1.0 + scroll.y * 0.001).clamp(0.9, 1.1);
                let new_zoom = (old_zoom * factor).clamp(0.05, 20.0);
                let scale = new_zoom / old_zoom;

                // Keep the point under the cursor fixed
                let p = pointer - rect.center();
                viewport.offset[0] = p.x * (1.0 - scale) + viewport.offset[0] * scale;
                viewport.offset[1] = p.y * (1.0 - scale) + viewport.offset[1] * scale;
                viewport.zoom = new_zoom;
            }
        }
    }

    // ── hover info ───────────────────────────────────────────
    let pointer = response.hover_pos()?;
    if !image_rect.contains(pointer) {
        return None;
    }

    let lx = (pointer.x - image_rect.left()) / viewport.zoom;
    let ly = (pointer.y - image_rect.top()) / viewport.zoom;
    if lx < 0.0 || ly < 0.0 {
        return None;
    }

    let x = lx.floor() as u32;
    let y = ly.floor() as u32;
    if x >= world_width || y >= world_height {
        return None;
    }

    Some(HoverInfo { x, y })
}
