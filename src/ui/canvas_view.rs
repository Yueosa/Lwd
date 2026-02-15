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
    let (rect, response) = ui.allocate_exact_size(available, Sense::drag());

    let checker = Color32::from_gray(28);
    let checker2 = Color32::from_gray(35);
    let tile = 24.0;
    let painter = ui.painter_at(rect);

    let cols = ((rect.width() / tile).ceil() as i32).max(1);
    let rows = ((rect.height() / tile).ceil() as i32).max(1);
    for row in 0..rows {
        for col in 0..cols {
            let min = Pos2::new(rect.left() + col as f32 * tile, rect.top() + row as f32 * tile);
            let max = Pos2::new(min.x + tile, min.y + tile);
            let color = if (row + col) % 2 == 0 { checker } else { checker2 };
            painter.rect_filled(Rect::from_min_max(min, max), 0.0, color);
        }
    }

    let image_size = Vec2::new(world_width as f32 * viewport.zoom, world_height as f32 * viewport.zoom);
    let center = rect.center() + Vec2::new(viewport.offset[0], viewport.offset[1]);
    let image_rect = Rect::from_center_size(center, image_size);

    painter.image(
        texture.id(),
        image_rect,
        Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );
    painter.rect_stroke(
        image_rect,
        0.0,
        Stroke::new(1.0, Color32::from_gray(120)),
    );

    if response.dragged() {
        let delta = response.drag_delta();
        viewport.pan(delta.x, delta.y);
    }

    if response.hovered() {
        let zoom_delta = ui.ctx().input(|i| i.zoom_delta());
        if (zoom_delta - 1.0).abs() > f32::EPSILON {
            viewport.zoom = (viewport.zoom * zoom_delta).clamp(0.1, 20.0);
        }
    }

    let pointer = response.hover_pos()?;
    if !image_rect.contains(pointer) {
        return None;
    }

    let local_x = (pointer.x - image_rect.left()) / viewport.zoom;
    let local_y = (pointer.y - image_rect.top()) / viewport.zoom;

    if local_x < 0.0 || local_y < 0.0 {
        return None;
    }

    let x = local_x.floor() as u32;
    let y = local_y.floor() as u32;
    if x >= world_width || y >= world_height {
        return None;
    }

    Some(HoverInfo { x, y })
}
