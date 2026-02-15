#[derive(Debug, Clone)]
pub struct ViewportState {
    pub zoom: f32,
    pub offset: [f32; 2],
}

impl Default for ViewportState {
    fn default() -> Self {
        Self {
            zoom: 0.3,
            offset: [0.0, 0.0],
        }
    }
}

impl ViewportState {
    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom * 1.2).min(20.0);
    }

    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom / 1.2).max(0.1);
    }

    pub fn reset(&mut self) {
        self.zoom = 0.3;
        self.offset = [0.0, 0.0];
    }

    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        self.offset[0] += delta_x;
        self.offset[1] += delta_y;
    }
}
