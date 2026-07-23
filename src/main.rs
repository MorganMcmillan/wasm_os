use raylib::prelude::*;
use tokio::task::{spawn_local, yield_now};
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

async fn create_kernel(rl: &mut RaylibHandle, thread: &RaylibThread) -> wasmtime::Result<Kernel> {
    // Generate default texture image
    let img = unsafe {
        raylib::ffi::GenImageColor(
            draw::FRAMEBUFFER_WIDTH as i32,
            draw::FRAMEBUFFER_HEIGHT as i32,
            Color::BLACK,
        )
    };
    let img = unsafe { Image::from_raw(img) };
    let texture = rl.load_texture_from_image(thread, &img).unwrap();

    let mut config = Config::new();
    config.wasm_component_model(true);
    config.strategy(Cranelift);
    let engine = Engine::new(&config)?;
    let drawstate = DrawState::new(texture);

    Ok(Kernel::new(engine, drawstate).await)
}

#[tokio::main(flavor = "local")]
async fn main() -> wasmtime::Result<()> {
    let (mut rl, thread) = raylib::init()
        .size(
            draw::FRAMEBUFFER_WIDTH as i32 * 2,
            draw::FRAMEBUFFER_HEIGHT as i32 * 2,
        )
        .title("WasmOS Test")
        .resizable()
        .build();

    rl.set_target_fps(20);

    let mut kernel = create_kernel(&mut rl, &thread).await?;

    let join_handle = spawn_local(async move {
        while !rl.window_should_close() {
            if kernel.root_exited() {
                println!("Root exited!");
                break;
            }

            kernel.update(&mut rl, &thread);
            kernel.upload_framebuffer();
            println!("Updating");

            yield_now().await;
        }
    });

    let _ = join_handle.await;

    Ok(())
}
