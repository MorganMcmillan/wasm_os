use raylib::RaylibHandle;

use crate::{
    KERNEL,
    driver::{Driver, draw},
    kernel::{ProcessContext, ProcessLinker},
};

/// Normalizes a given coordinate to be within `normalized_length`.
fn normalize_coordinate(x: i32, length: i32, normalized_length: i32) -> u16 {
    ((x * normalized_length) / length) as u16
}

#[derive(Debug)]
pub struct MouseState {
    pub x: u16,
    pub y: u16,
    driver_id: usize,
}

impl MouseState {
    pub fn new() -> Self {
        Self {
            x: 0,
            y: 0,
            driver_id: 0,
        }
    }
}

impl Driver for MouseState {
    fn register_functions(&self, linker: &mut ProcessLinker) -> wasmtime::Result<()> {
        let id = self.driver_id;

        linker.func_wrap("get_mouse_x", move |_: ProcessContext, _: ()| unsafe {
            let mousestate = KERNEL.get_driver::<Self>(id);
            Ok((mousestate.x as i32,))
        })?;

        linker.func_wrap("get_mouse_y", move |_: ProcessContext, _: ()| unsafe {
            let mousestate = KERNEL.get_driver::<Self>(id);
            Ok((mousestate.y as i32,))
        })?;

        Ok(())
    }

    fn update(&mut self, rl: &mut RaylibHandle, _thread: &raylib::RaylibThread) {
        let screen_width = rl.get_screen_width();
        let screen_height = rl.get_screen_height();
        let mx = rl.get_mouse_x();
        let my = rl.get_mouse_y();

        self.x = normalize_coordinate(mx, screen_width, draw::FRAMEBUFFER_WIDTH as i32);
        self.y = normalize_coordinate(my, screen_height, draw::FRAMEBUFFER_HEIGHT as i32);
    }

    fn accept_id(&mut self, id: usize) {
        self.driver_id = id;
    }

    fn get_id(&self) -> usize {
        self.driver_id
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
