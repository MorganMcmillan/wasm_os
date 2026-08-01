use crate::{
    driver::{Driver, RaylibUserdata},
    kernel::{Kernel, ProcessContext, ProcessLinker},
    mut_cell::MutCell,
    system_functions,
};
use raylib::{
    drawing::{RaylibDraw, RaylibDrawHandle},
    ffi::{Color, KeyboardKey, Vector2},
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

pub struct ScreenState {
    pub framebuffer_texture: Texture2D,
}

impl ScreenState {
    pub fn new(texture: Texture2D) -> Self {
        Self {
            framebuffer_texture: texture,
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

impl Driver<RaylibUserdata> for ScreenState {
    fn name(&self) -> &'static str {
        "driver_screen"
    }

    fn register_functions(
        &self,
        linker: &mut ProcessLinker<RaylibUserdata>,
        id: usize,
    ) -> wasmtime::Result<()> {
        let name = self.name();

        linker.func_wrap(
            name,
            "upload_framebuffer",
            move |ctx: ProcessContext<RaylibUserdata>, framebuffer: i32| {
                let drawstate = ctx.data().kernel.borrow_static().get_driver::<Self>(id);
                drawstate.upload_framebuffer(system_functions::get_memory(
                    &ctx,
                    framebuffer,
                    FRAMEBUFFER_SIZE as u32,
                ));
                Ok(())
            },
        )?;

        Ok(())
    }

    fn update(
        &mut self,
        _kernel: &'static MutCell<Kernel<RaylibUserdata>>,
        (rl, thread): &mut RaylibUserdata,
    ) {
        if rl.is_key_pressed(KeyboardKey::KEY_F11) {
            rl.toggle_fullscreen();
        }

        let screen_width = rl.get_screen_width();
        let screen_height = rl.get_screen_height();

        let mut d = rl.begin_drawing(thread);
        self.draw_framebuffer(&mut d, screen_width, screen_height);
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
