#![allow(static_mut_refs)]

use raylib::prelude::*;
use tokio::task::{spawn_local, yield_now};
use wasmtime::{Strategy::Cranelift, *};

use crate::draw::DrawState;
use crate::kernel::Kernel;
use crate::option_cell::OptionCell;

mod draw;
mod event;
mod input;
mod kernel;
mod option_cell;
mod process;
mod ptr_cell;
mod wasm_state;

const FRAMERATE: u32 = 60;

static mut KERNEL: OptionCell<Kernel> = const { OptionCell::none() };

async fn create_kernel(rl: &mut RaylibHandle, thread: &RaylibThread) -> wasmtime::Result<Kernel> {
    // Generate default texture image
    let img = unsafe {
        raylib::ffi::GenImageColor(
            draw::FRAMEBUFFER_WIDTH as i32,
            draw::FRAMEBUFFER_HEIGHT as i32,
            Color::RED,
        )
    };
    let img = unsafe { Image::from_raw(img) };
    let texture = rl.load_texture_from_image(thread, &img).unwrap();

    let mut config = Config::new();
    config.strategy(Cranelift);
    let engine = Engine::new(&config)?;
    let drawstate = DrawState::new(texture);

    Ok(Kernel::new(engine, drawstate))
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

    rl.set_target_fps(FRAMERATE);

    unsafe {
        KERNEL = OptionCell::new(create_kernel(&mut rl, &thread).await?);
        KERNEL.run_boot().await;

        let join_handle = spawn_local(async move {
            while !rl.window_should_close() {
                if KERNEL.root_exited() {
                    break;
                }

                let screen_width = rl.get_screen_width();
                let screen_height = rl.get_screen_height();

                KERNEL.update(&mut rl);
                // println!("Test data kernel: {}!", kernel.test_data);
                // println!(
                //     "Test data boot: {}!",
                //     kernel
                //         .processes
                //         .first()
                //         .as_ref()
                //         .unwrap()
                //         .as_ref()
                //         .unwrap()
                //         .wasm_state
                //         .kernel()
                //         .test_data
                // );
                KERNEL.upload_framebuffer();

                let mut d = rl.begin_drawing(&thread);
                KERNEL
                    .drawstate
                    .draw_framebuffer(&mut d, screen_width, screen_height);

                yield_now().await;
            }
        });

        let _ = join_handle.await;
    }

    Ok(())
}
