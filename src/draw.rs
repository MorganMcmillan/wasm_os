use crate::kernel::Pid;
use raylib::{
    drawing::{RaylibDraw, RaylibDrawHandle},
    ffi::{Color, Vector2},
    math::Rectangle,
    texture::{RaylibTexture2D, Texture2D},
};

pub const FRAMEBUFFER_WIDTH: usize = 384;
pub const FRAMEBUFFER_HEIGHT: usize = 216;
pub const FRAMEBUFFER_SIZE: usize = FRAMEBUFFER_WIDTH * FRAMEBUFFER_HEIGHT;

/// Takes a byte in the format `0bRRRGGGBB` and returns an RGB tuple, normalized to `[0, 255]`.
fn byte_to_rgb(byte: u8) -> (u8, u8, u8) {
    let r: u8 = (byte & 0b11100000) >> 5;
    let g: u8 = (byte & 0b00011100) >> 2;
    let b: u8 = byte & 0b00000011;
    (r.saturating_mul(37), g.saturating_mul(37), b * 85)
}

pub struct DrawState {
    pub was_set: bool,
    pub framebuffer_address: Option<(Pid, u32)>,
    pub framebuffer_texture: Texture2D,
}

impl DrawState {
    pub fn new(texture: Texture2D) -> Self {
        Self {
            was_set: false,
            framebuffer_address: None,
            framebuffer_texture: texture,
        }
    }

    pub fn set_framebuffer_address(&mut self, pid: Pid, mem_address: u32) {
        self.framebuffer_address = Some((pid, mem_address));
    }

    pub fn upload_framebuffer(&mut self, framebuffer: &[u8]) {
        let mut fb_rgba8888 = [0u8; FRAMEBUFFER_WIDTH * FRAMEBUFFER_HEIGHT * 4];
        for (i, pixel) in framebuffer.iter().enumerate() {
            let (r, g, b) = byte_to_rgb(*pixel);
            fb_rgba8888[i * 4] = r;
            fb_rgba8888[i * 4 + 1] = g;
            fb_rgba8888[i * 4 + 2] = b;
            fb_rgba8888[i * 4 + 3] = 255;
        }
        self.framebuffer_texture
            .update_texture(&fb_rgba8888)
            .unwrap();
    }

    pub fn draw_framebuffer(
        &self,
        d: &mut RaylibDrawHandle,
        screen_width: i32,
        screen_height: i32,
    ) {
        println!("Drawing framebuffer!");
        d.draw_texture_pro(
            &self.framebuffer_texture,
            Rectangle::new(
                0.0,
                0.0,
                FRAMEBUFFER_WIDTH as f32,
                FRAMEBUFFER_HEIGHT as f32,
            ),
            Rectangle::new(0.0, 0.0, screen_width as f32, screen_height as f32),
            Vector2::new(0.0, 0.0),
            0.0,
            Color::WHITE,
        );
    }
}
