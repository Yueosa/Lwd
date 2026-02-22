use std::sync::{Arc, Mutex};

use egui::{Color32, ColorImage, Pos2, Rect, Sense, Stroke, TextureHandle, Ui, Vec2};
use rayon::prelude::*;

use crate::core::biome::{BiomeDefinition, BiomeMap};
use crate::core::layer::LayerDefinition;
use crate::core::world::World;
use crate::rendering::canvas::world_to_color_image_region_lod;
use crate::rendering::gl_canvas::{GlCanvasParams, GlCanvasState, make_canvas_callback, pixels_to_rgba};
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

/// 从 BiomeMap 的子区域 [rx, ry, rw×rh] 生成半透明 overlay 纹理，按 LOD 降采样
fn biome_overlay_image_region_lod(
    biome_map: &BiomeMap,
    biome_definitions: &[BiomeDefinition],
    rx: u32,
    ry: u32,
    rw: u32,
    rh: u32,
    lod: u32,
) -> ColorImage {
    let f = lod.max(1) as usize;
    let rw = rw as usize;
    let rh = rh as usize;
    let rx = rx as usize;
    let ry = ry as usize;
    let bw = biome_map.width as usize;

    let out_w = (rw + f - 1) / f;
    let out_h = (rh + f - 1) / f;

    let mut biome_lut = [Color32::TRANSPARENT; 256];
    for bdef in biome_definitions {
        let c = bdef.overlay_color;
        biome_lut[bdef.id as usize] = Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
    }

    let data = biome_map.data();
    let mut pixels = vec![Color32::TRANSPARENT; out_w * out_h];

    pixels
        .par_chunks_mut(out_w)
        .enumerate()
        .for_each(|(out_row, row_pixels)| {
            let src_y = ry + out_row * f;
            let src_row_start = src_y * bw;
            for out_x in 0..out_w {
                let src_x = rx + out_x * f;
                let idx = src_row_start + src_x;
                if idx < data.len() {
                    row_pixels[out_x] = biome_lut[data[idx] as usize];
                }
            }
        });

    ColorImage {
        size: [out_w, out_h],
        pixels,
    }
}

/// 在 biome overlay 上找到各区域的中心并标注名称。
///
/// 仅扫描当前**可见视口区域**的 biome 采样点（而非全世界），
/// 并根据缩放级别使用自适应采样步长，确保在大世界缩小查看时
/// 也能保持低开销（<2ms）。
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

    let w = biome_map.width;
    let h = biome_map.height;

    // ── 只扫描可见视口范围（+边距），避免遍历整个世界 ──
    let clip = painter.clip_rect();
    let margin_world = 50u32; // 边距：世界像素
    let vis_x0 = ((clip.left() - image_rect.left()) / zoom)
        .max(0.0) as u32;
    let vis_y0 = ((clip.top() - image_rect.top()) / zoom)
        .max(0.0) as u32;
    let vis_x1 = ((clip.right() - image_rect.left()) / zoom)
        .ceil().min(w as f32) as u32;
    let vis_y1 = ((clip.bottom() - image_rect.top()) / zoom)
        .ceil().min(h as f32) as u32;

    let x_start = vis_x0.saturating_sub(margin_world);
    let x_end = (vis_x1 + margin_world).min(w);
    let y_start = vis_y0.saturating_sub(margin_world);
    let y_end = (vis_y1 + margin_world).min(h);

    if x_end <= x_start || y_end <= y_start {
        return;
    }

    // ── 自适应步长：缩放越小 → 步长越大（因为屏幕上细节更少） ──
    // zoom=0.3 → step=48, zoom=0.5 → step=32, zoom=1.0+ → step=16
    let step = if zoom < 0.4 { 48u32 } else if zoom < 0.8 { 32u32 } else { 16u32 };

    // 对每种 biome 收集可见区域内的采样点 (x, y)，跳过 UNASSIGNED
    let mut sample_map: HashMap<u8, Vec<(u32, u32)>> = HashMap::new();

    let mut y = y_start;
    while y < y_end {
        let mut x = x_start;
        while x < x_end {
            let bid = biome_map.get(x, y);
            if bid != 0 {
                sample_map.entry(bid).or_default().push((x, y));
            }
            x += step;
        }
        y += step;
    }

    // 间隔阈值：如果连续采样点之间 x 间距超过此值，视为两个独立区域
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
            // 只保留落在可见区域内的候选标签
            if clip.contains(pos) {
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

    // 限制最大候选数量（避免极端情况下过多 layout 调用）
    candidates.truncate(32);

    // 第二阶段：逐个放置，碰撞检测
    let font = egui::FontId::proportional(14.0);
    let mut placed_rects: Vec<Rect> = Vec::new();
    let label_padding = 4.0_f32;

    // 预计算一个通用中文字符的大致高度，用于快速碰撞估算
    // （避免为每个候选都做完整 layout）
    let sample_galley = painter.layout_no_wrap("测".into(), font.clone(), Color32::WHITE);
    let char_h = sample_galley.size().y;

    for candidate in &candidates {
        // 用字符数估算宽度（中文约 char_h 宽，ASCII 约 char_h*0.6）
        let approx_w = candidate.text.chars().count() as f32 * char_h * 0.7;
        let half_w = approx_w / 2.0 + label_padding;
        let half_h = char_h / 2.0 + label_padding;

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
        let dy_step = char_h + 6.0;
        let dx_step = approx_w * 0.6 + 6.0;
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
            if !clip.contains(pos) {
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
    world: &World,
    color_lut: &[Color32; 256],
    viewport: &mut ViewportState,
    biome_map: Option<&BiomeMap>,
    biome_definitions: &[BiomeDefinition],
    layers: &[LayerDefinition],
    show_biome_color: bool,
    show_biome_labels: bool,
    show_layer_lines: bool,
    show_layer_labels: bool,
    gl_canvas: &Arc<Mutex<GlCanvasState>>,
) -> Option<HoverInfo> {
    let world_width = world.width;
    let world_height = world.height;

    let available = ui.available_size();
    let (rect, response) = ui.allocate_exact_size(available, Sense::click_and_drag());

    // ── world image rect (full world in screen coords) ───────
    let image_size = Vec2::new(
        world_width as f32 * viewport.zoom,
        world_height as f32 * viewport.zoom,
    );
    let center = rect.center() + Vec2::new(viewport.offset[0], viewport.offset[1]);
    let image_rect = Rect::from_center_size(center, image_size);

    // ── viewport culling: compute visible world region ───────
    let vis_left = ((rect.left() - image_rect.left()) / viewport.zoom)
        .max(0.0)
        .floor() as u32;
    let vis_top = ((rect.top() - image_rect.top()) / viewport.zoom)
        .max(0.0)
        .floor() as u32;
    let vis_right = ((rect.right() - image_rect.left()) / viewport.zoom)
        .min(world_width as f32)
        .ceil() as u32;
    let vis_bottom = ((rect.bottom() - image_rect.top()) / viewport.zoom)
        .min(world_height as f32)
        .ceil() as u32;

    let vis_x = vis_left.min(world_width);
    let vis_y = vis_top.min(world_height);
    let vis_w = vis_right.saturating_sub(vis_left).min(world_width - vis_x);
    let vis_h = vis_bottom.saturating_sub(vis_top).min(world_height - vis_y);

    // ── dynamic LOD: when zoomed out, downsample to match screen resolution ──
    // zoom=0.3 → each screen pixel covers ~3 world pixels → LOD 3
    // zoom=1.0 → 1:1 → LOD 1 (full resolution)
    // zoom=5.0 → zoomed in → LOD 1 (full resolution, small region)
    let lod = if viewport.zoom < 1.0 {
        (1.0 / viewport.zoom).floor().max(1.0).min(8.0) as u32
    } else {
        1u32
    };

    // Expand to 3× buffer for comfortable panning headroom
    // Align buffer boundaries to LOD grid for consistent sampling
    let align = |v: u32| (v / lod) * lod;
    let buf_x = align(vis_x.saturating_sub(vis_w));
    let buf_y = align(vis_y.saturating_sub(vis_h));
    let buf_right = (vis_x + vis_w * 2).min(world_width);
    let buf_bottom = (vis_y + vis_h * 2).min(world_height);
    let buf_w = buf_right - buf_x;
    let buf_h = buf_bottom - buf_y;

    let visible_region = [vis_x, vis_y, vis_w, vis_h];
    let buffer_region = [buf_x, buf_y, buf_w, buf_h];

    // ── re-render region pixels if needed (viewport moved / world changed / LOD changed)
    {
        let needs_update = gl_canvas.lock().unwrap().needs_region_update(visible_region, lod);
        if needs_update && buf_w > 0 && buf_h > 0 {
            let img = world_to_color_image_region_lod(
                world, color_lut,
                buffer_region[0], buffer_region[1],
                buffer_region[2], buffer_region[3],
                lod,
            );
            let tex_w = img.size[0] as u32;
            let tex_h = img.size[1] as u32;
            let rgba = pixels_to_rgba(&img.pixels);
            gl_canvas.lock().unwrap().set_world_region_pixels(
                rgba, tex_w, tex_h,
                buffer_region, lod,
            );
        }
    }

    // ── biome overlay for current region ─────────────────────
    if show_biome_color {
        if let Some(bm) = biome_map {
            let st = gl_canvas.lock().unwrap();
            let cur_region = st.world_region().unwrap_or(buffer_region);
            let cur_lod = st.current_lod();
            let needs_regen = st.needs_biome_regen(cur_region, cur_lod);
            drop(st);
            if needs_regen && cur_region[2] > 0 && cur_region[3] > 0 {
                let img = biome_overlay_image_region_lod(
                    bm,
                    biome_definitions,
                    cur_region[0], cur_region[1],
                    cur_region[2], cur_region[3],
                    cur_lod,
                );
                let tex_w = img.size[0] as u32;
                let tex_h = img.size[1] as u32;
                let rgba = pixels_to_rgba(&img.pixels);
                gl_canvas.lock().unwrap().set_biome_region_pixels(
                    rgba, tex_w, tex_h,
                    cur_region, cur_lod,
                );
            }
        }
    }

    // ── GL PaintCallback (checkerboard + world region + biome) ──
    {
        // Map the currently buffered region to screen coords
        let region = gl_canvas.lock().unwrap().world_region().unwrap_or(buffer_region);

        let region_screen_left = image_rect.left() + region[0] as f32 * viewport.zoom;
        let region_screen_top = image_rect.top() + region[1] as f32 * viewport.zoom;
        let region_screen_right = region_screen_left + region[2] as f32 * viewport.zoom;
        let region_screen_bottom = region_screen_top + region[3] as f32 * viewport.zoom;

        let rw = rect.width().max(1.0);
        let rh = rect.height().max(1.0);
        let world_rect_norm = [
            (region_screen_left - rect.left()) / rw,
            (region_screen_top - rect.top()) / rh,
            (region_screen_right - rect.left()) / rw,
            (region_screen_bottom - rect.top()) / rh,
        ];
        let has_biome_flag =
            show_biome_color && gl_canvas.lock().unwrap().has_biome_ready();

        let callback = make_canvas_callback(
            Arc::clone(gl_canvas),
            GlCanvasParams {
                canvas_rect: rect,
                world_rect_norm,
                has_world: true,
                has_biome: has_biome_flag,
            },
        );
        ui.painter().add(callback);
    }

    // ── border around world image (full world extent) ────────
    let painter = ui.painter_at(rect);
    painter.rect_stroke(image_rect, 0.0, Stroke::new(1.0, Color32::from_gray(120)));

    // ── biome labels (lightweight egui text) ─────────────────
    if show_biome_labels {
        if let Some(bm) = biome_map {
            draw_biome_labels(&painter, bm, biome_definitions, image_rect, viewport.zoom);
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
        let scroll = ui.ctx().input(|i| i.raw_scroll_delta);
        if scroll.y.abs() > 0.5 {
            if let Some(pointer) = ui.ctx().input(|i| i.pointer.hover_pos()) {
                let old_zoom = viewport.zoom;
                // raw_scroll_delta: ~120px/notch (mouse) or smaller (touchpad)
                // 0.001 × 120 = 0.12 → clamped to ±10% per single scroll event
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
