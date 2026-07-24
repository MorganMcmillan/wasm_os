use raylib::RaylibHandle;

use crate::draw;

/// Normalizes a given coordinate to be within `normalized_length`.
fn normalize_coordinate(x: i32, length: i32, normalized_length: i32) -> u16 {
    ((x * normalized_length) / length) as u16
}

#[derive(Debug)]
pub struct MouseState {
    pub x: u16,
    pub y: u16,
}

impl MouseState {
    pub fn new() -> Self {
        Self { x: 0, y: 0 }
    }

    pub fn update(&mut self, rl: &mut RaylibHandle) {
        let screen_width = rl.get_screen_width();
        let screen_height = rl.get_screen_height();
        let mx = rl.get_mouse_x();
        let my = rl.get_mouse_y();
        self.x = normalize_coordinate(mx, screen_width, draw::FRAMEBUFFER_WIDTH as i32);
        self.y = normalize_coordinate(my, screen_height, draw::FRAMEBUFFER_HEIGHT as i32);
    }
}

