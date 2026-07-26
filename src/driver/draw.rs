use crate::{
    KERNEL,
    driver::Driver,
    kernel::{ProcessContext, ProcessLinker},
};
use raylib::{
    drawing::{RaylibDraw, RaylibDrawHandle},
    ffi::{Color, Vector2},
    math::Rectangle,
    texture::{RaylibTexture2D, Texture2D},
};
use wasmtime::component::WasmList;

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
    pub framebuffer_texture: Texture2D,
    driver_id: usize,
}

impl DrawState {
    pub fn new(texture: Texture2D) -> Self {
        Self {
            framebuffer_texture: texture,
            driver_id: 0,
        }
    }

    /// Uploads the process' framebuffer into the framebuffer texture, mapping each byte to
    /// rgba8888.
    pub fn upload_framebuffer(&mut self, mut framebuffer: &[u8]) {
        if framebuffer.len() != FRAMEBUFFER_SIZE {
            eprintln!(
                "Warning: provided framebuffer's size is not as expected: got {}",
                framebuffer.len()
            );
            if framebuffer.len() > FRAMEBUFFER_SIZE {
                framebuffer = &framebuffer[..FRAMEBUFFER_SIZE];
            }
        }

        let mut fb_rgba8888 = [0u8; FRAMEBUFFER_SIZE * 4];
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

impl Driver for DrawState {
    fn accept_id(&mut self, id: usize) {
        self.driver_id = id;
    }

    fn get_id(&self) -> usize {
        self.driver_id
    }

    fn register_functions(&self, linker: &mut ProcessLinker) -> wasmtime::Result<()> {
        let id = self.driver_id;

        linker.func_wrap(
            "upload_framebuffer",
            move |ctx: ProcessContext, (framebuffer,): (WasmList<u8>,)| {
                unsafe {
                    let drawstate = KERNEL.get_driver::<Self>(id);
                    drawstate.upload_framebuffer(framebuffer.as_le_slice(&ctx));
                }
                Ok(())
            },
        )?;

        Ok(())
    }

    fn update(&mut self, rl: &mut raylib::RaylibHandle, thread: &raylib::RaylibThread) {
        // unsafe {
        //     if let Some((pid, address)) = self.framebuffer_address {
        //         let process = KERNEL.get_process(pid).unwrap();
        //         let framebuffer = process.get_memory(address as usize, FRAMEBUFFER_SIZE);
        //         self.upload_framebuffer(framebuffer);
        //     } else {
        //         eprintln!("Warning: no framebuffer set!");
        //     }
        // }

        // TODO: create a systsem function called `upload_framebuffer` to update the framebuffer
        // when needed.
        let screen_width = rl.get_screen_width();
        let screen_height = rl.get_screen_height();

        let mut d = rl.begin_drawing(thread);
        self.draw_framebuffer(&mut d, screen_width, screen_height);
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
