#![allow(static_mut_refs)]

use raylib::prelude::*;
use tokio::task::{spawn_local, yield_now};

use crate::driver::audio::AudioState;
use crate::driver::input::InputState;
use crate::driver::screen::ScreenState;
use crate::kernel::Kernel;
use crate::mut_cell::MutCell;

mod async_file;
mod byte_builder;
mod driver;
mod event;
mod id;
mod kernel;
mod mut_cell;
mod process;
mod ptr_cell;
mod system_functions;
mod wasm_process;

const APP_NAME: &str = "wasm_os";
// const FRAMERATE: u32 = 120;
const DISABLE_LOGGING: bool = true;

async fn create_kernel(rl: &mut RaylibHandle, thread: &RaylibThread) -> wasmtime::Result<Kernel> {
    // Generate default texture image
    let img = unsafe {
        raylib::ffi::GenImageColor(
            driver::screen::FRAMEBUFFER_WIDTH as i32,
            driver::screen::FRAMEBUFFER_HEIGHT as i32,
            Color::RED,
        )
    };
    let img = unsafe { Image::from_raw(img) };
    let texture = rl.load_texture_from_image(thread, &img).unwrap();

    let root_dir = app_dirs2::app_root(
        app_dirs2::AppDataType::UserData,
        &app_dirs2::AppInfo {
            name: APP_NAME,
            author: "Morgan",
        },
    )
    .expect("Could not create application directory.");

    let drawstate = Box::new(ScreenState::new(texture));
    let inputstate = Box::new(InputState::new());
    let audiostate = Box::new(AudioState::new());

    Ok(Kernel::new(
        &root_dir,
        vec![drawstate, inputstate, audiostate],
    ))
}

#[tokio::main(flavor = "local")]
async fn main() -> wasmtime::Result<()> {
    // Disable raylib's logging
    if DISABLE_LOGGING {
        let _ = set_trace_log_callback(|_, _| {});
    }

    let (mut rl, thread) = raylib::init()
        .size(
            driver::screen::FRAMEBUFFER_WIDTH as i32 * 2,
            driver::screen::FRAMEBUFFER_HEIGHT as i32 * 2,
        )
        .vsync()
        .title("WasmOS Test")
        .resizable()
        .build();

    rl.hide_cursor();

    // rl.set_target_fps(FRAMERATE);

    let kernel_cell = MutCell::new(create_kernel(&mut rl, &thread).await?);
    let kernel = kernel_cell.as_static_ref();

    Kernel::run_boot(kernel).await;

    let join_handle = spawn_local(async move {
        while !rl.window_should_close() {
            if kernel.root_exited() {
                println!("Root exited, shutting down.");
                break;
            }

            Kernel::update(kernel, &mut rl, &thread);

            yield_now().await;
        }
    });

    let _ = join_handle.await;

    Ok(())
}
