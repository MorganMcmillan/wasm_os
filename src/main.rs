use raylib::prelude::*;
use wasmtime::{Strategy::Cranelift, *};

use crate::draw::DrawState;

mod draw;
mod input;
mod kernel;
mod ptr_cell;

use kernel::*;

fn main() -> wasmtime::Result<()> {
    let (mut rl, thread) = raylib::init()
        .size(
            draw::FRAMEBUFFER_WIDTH as i32 * 2,
            draw::FRAMEBUFFER_HEIGHT as i32 * 2,
        )
        .title("WasmOS Test")
        .resizable()
        .build();

    rl.set_target_fps(20);

    // Generate default texture image
    let img = unsafe {
        raylib::ffi::GenImageColor(
            draw::FRAMEBUFFER_WIDTH as i32,
            draw::FRAMEBUFFER_HEIGHT as i32,
            Color::BLACK,
        )
    };
    let img = unsafe { Image::from_raw(img) };
    let texture = rl.load_texture_from_image(&thread, &img).unwrap();

    let mut config = Config::new();
    config.strategy(Cranelift);
    let engine = Engine::new(&config)?;
    let drawstate = DrawState::new(texture);
    let kernel = Kernel::new(drawstate);

    let mut wasmstate = WasmState::new(&engine, kernel)?;
    wasmstate.init().unwrap();

    while !rl.window_should_close() {
        wasmstate.update();
        wasmstate.upload_framebuffer();

        if rl.is_key_pressed(KeyboardKey::KEY_F11) {
            rl.toggle_fullscreen();
        }

        let screen_width = rl.get_screen_width();
        let screen_height = rl.get_screen_height();
        let mx = rl.get_mouse_x();
        let my = rl.get_mouse_y();

        wasmstate
            .kernel_mut()
            .mousestate
            .update(mx, my, screen_width, screen_height);

        let mut d = rl.begin_drawing(&thread);
        wasmstate
            .kernel()
            .drawstate
            .draw_framebuffer(&mut d, screen_width, screen_height);
    }

    Ok(())
}
