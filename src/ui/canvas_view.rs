use egui::{Color32, Pos2, Rect, Sense, Stroke, TextureHandle, Ui, Vec2};

use crate::rendering::viewport::ViewportState;

#[derive(Debug, Clone, Copy)]
pub struct HoverInfo {
    pub x: u32,
    pub y: u32,
}

pub fn show_canvas(
    ui: &mut Ui,
    texture: &TextureHandle,
    world_width: u32,
    world_height: u32,
    viewport: &mut ViewportState,
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
