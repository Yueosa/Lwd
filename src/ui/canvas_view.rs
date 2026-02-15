use egui::{Color32, ColorImage, Pos2, Rect, Sense, Stroke, TextureHandle, TextureOptions, Ui, Vec2};

use crate::core::biome::{BiomeDefinition, BiomeMap};
use crate::core::layer::LayerDefinition;
use crate::rendering::viewport::ViewportState;

#[derive(Debug, Clone, Copy)]
pub struct HoverInfo {
    pub x: u32,
    pub y: u32,
}

/// 从 2D BiomeMap 生成半透明 overlay 纹理
fn biome_overlay_image(
    biome_map: &BiomeMap,
    biome_definitions: &[BiomeDefinition],
) -> ColorImage {
    let w = biome_map.width as usize;
    let h = biome_map.height as usize;
    let mut pixels = vec![Color32::TRANSPARENT; w * h];

    for y in 0..h {
        for x in 0..w {
            let bid = biome_map.get(x as u32, y as u32);
            if let Some(bdef) = biome_definitions.iter().find(|b| b.id == bid) {
                let c = bdef.overlay_color;
                pixels[y * w + x] = Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
            }
        }
    }

    ColorImage {
        size: [w, h],
        pixels,
    }
}

/// 在 biome overlay 上找到各区域的中心并标注名称。
///
/// 对于分布在世界两侧的环境（如海洋），会检测不连续区域并分别标注。
fn draw_biome_labels(
    painter: &egui::Painter,
    biome_map: &BiomeMap,
    biome_definitions: &[BiomeDefinition],
    image_rect: Rect,
    zoom: f32,
) {
    use std::collections::HashMap;

    // 对每种 biome 收集所有采样点的 x 坐标列表和 y 范围
    struct SampleData {
        xs: Vec<u32>,
        sum_y: u64,
        count: u64,
    }
    let mut sample_map: HashMap<u8, SampleData> = HashMap::new();

    let step = 8u32;
    let w = biome_map.width;
    let h = biome_map.height;
    let mut y = 0;
    while y < h {
        let mut x = 0;
        while x < w {
            let bid = biome_map.get(x, y);
            sample_map
                .entry(bid)
                .and_modify(|s| {
                    s.xs.push(x);
                    s.sum_y += y as u64;
                    s.count += 1;
                })
                .or_insert(SampleData {
                    xs: vec![x],
                    sum_y: y as u64,
                    count: 1,
                });
            x += step;
        }
        y += step;
    }

    // 间隔阈值：如果连续采样列之间的间距超过此值，视为两个独立区域
    let gap_threshold = w / 5;

    for (bid, data) in &sample_map {
        let bdef = match biome_definitions.iter().find(|d| d.id == *bid) {
            Some(d) => d,
            None => continue,
        };

        let avg_y = data.sum_y as f32 / data.count as f32;

        // 将 x 排序后按间隔切分成独立区域
        let mut sorted_xs = data.xs.clone();
        sorted_xs.sort_unstable();

        let mut regions: Vec<(u32, u32)> = Vec::new(); // (min_x, max_x)
        let mut region_start = sorted_xs[0];
        let mut region_end = sorted_xs[0];

        for &x in &sorted_xs[1..] {
            if x - region_end > gap_threshold {
                regions.push((region_start, region_end));
                region_start = x;
            }
            region_end = x;
        }
        regions.push((region_start, region_end));

        // 在每个区域的中心放置标签
        for (rx_min, rx_max) in &regions {
            let cx = ((*rx_min + *rx_max) as f32 / 2.0) * zoom + image_rect.left();
            let cy = avg_y * zoom + image_rect.top();
            let pos = Pos2::new(cx, cy);
            if image_rect.contains(pos) {
                painter.text(
                    pos,
                    egui::Align2::CENTER_CENTER,
                    &bdef.name,
                    egui::FontId::proportional(14.0),
                    Color32::WHITE,
                );
            }
        }
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
    show_biome_overlay: bool,
    show_layer_overlay: bool,
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
    if show_biome_overlay {
        if let Some(bm) = biome_map {
            // 惰性生成 biome overlay 纹理
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

            // 绘制环境名称标签
            draw_biome_labels(&painter, bm, biome_definitions, image_rect, viewport.zoom);
        }
    }

    // ── layer overlay ──────────────────────────────────────────
    if show_layer_overlay {
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
        
        // 在每个层级的中心位置绘制名称标签
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
