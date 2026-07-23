use raylib::prelude::*;
use wasmtime::{Strategy::Cranelift, *};

use crate::draw::DrawState;

mod draw;
mod event;
mod input;
mod kernel;
mod process;
mod ptr_cell;
mod wasm_state;

use kernel::Kernel;

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
    let mut kernel = Kernel::new(engine, drawstate);

    while !rl.window_should_close() {
        kernel.update(&mut rl, &thread);
        kernel.upload_framebuffer();
    }

    Ok(())
}
