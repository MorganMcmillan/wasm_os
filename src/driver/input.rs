use raylib::ffi::MouseButton;

use crate::{
    byte_builder::ByteBuilder,
    driver::{Driver, RaylibUserdata, screen},
    kernel::{Kernel, ProcessContext, ProcessLinker},
    mut_cell::MutCell,
};

/// Normalizes a given coordinate to be within `normalized_length`.
fn normalize_coordinate(x: i32, length: i32, normalized_length: i32) -> u16 {
    ((x * normalized_length) / length) as u16
}

fn send_mouse_event(
    kernel: &MutCell<Kernel<RaylibUserdata>>,
    name: &str,
    mx: u16,
    my: u16,
    button: u8,
) {
    kernel
        .borrow_static()
        .send_event_to_root(name, &ByteBuilder::new().u16(mx).u16(my).u8(button).build());
}

pub struct InputState {
    pub x: u16,
    pub y: u16,
}

impl InputState {
    pub fn new() -> Self {
        Self { x: 0, y: 0 }
    }
}

impl Driver<RaylibUserdata> for InputState {
    fn name(&self) -> &'static str {
        "driver_input"
    }

    fn register_functions(
        &self,
        linker: &mut ProcessLinker<RaylibUserdata>,
        id: usize,
    ) -> wasmtime::Result<()> {
        let name = self.name();

        linker.func_wrap(
            name,
            "get_mouse_x",
            move |ctx: ProcessContext<RaylibUserdata>| {
                let mousestate = ctx.data().kernel.borrow_static().get_driver::<Self>(id);
                mousestate.x as i32
            },
        )?;

        linker.func_wrap(
            name,
            "get_mouse_y",
            move |ctx: ProcessContext<RaylibUserdata>| {
                let mousestate = ctx.data().kernel.borrow_static().get_driver::<Self>(id);
                mousestate.y as i32
            },
        )?;

        Ok(())
    }

    fn update(
        &mut self,
        kernel: &MutCell<Kernel<RaylibUserdata>>,
        (rl, _thread): &mut RaylibUserdata,
    ) {
        let screen_width = rl.get_screen_width();
        let screen_height = rl.get_screen_height();
        let mx = rl.get_mouse_x();
        let my = rl.get_mouse_y();

        let mx = normalize_coordinate(mx, screen_width, screen::FRAMEBUFFER_WIDTH as i32);
        let my = normalize_coordinate(my, screen_height, screen::FRAMEBUFFER_HEIGHT as i32);
        self.x = mx;
        self.y = my;

        // Send keyboard events

        // Iterate all raylib keys
        // for i in 0..=336 {
        //     unsafe {
        //         let key = transmute::<i32, KeyboardKey>(i);
        //
        //         if rl.is_key_pressed(key) {
        //             KERNEL.send_event_to_root("key_pressed", &(i as u16).to_le_bytes());
        //         }
        //         if rl.is_key_released(key) {
        //             KERNEL.send_event_to_root("key_pressed", &(i as u16).to_le_bytes());
        //         }
        //     }
        // }

        while let Some(c) = rl.get_char_pressed() {
            if c.is_ascii() {
                kernel
                    .borrow_static()
                    .send_event_to_root("char", &[c as u8]);
            }
        }

        // Send mouse click events

        const MOUSE_BUTTON_NUMBERS: [(MouseButton, u8); 7] = [
            (MouseButton::MOUSE_BUTTON_LEFT, 0),
            (MouseButton::MOUSE_BUTTON_RIGHT, 1),
            (MouseButton::MOUSE_BUTTON_MIDDLE, 2),
            (MouseButton::MOUSE_BUTTON_BACK, 3),
            (MouseButton::MOUSE_BUTTON_FORWARD, 4),
            (MouseButton::MOUSE_BUTTON_SIDE, 5),
            (MouseButton::MOUSE_BUTTON_EXTRA, 6),
        ];

        for (mouse_button, number) in MOUSE_BUTTON_NUMBERS {
            if rl.is_mouse_button_pressed(mouse_button) {
                send_mouse_event(kernel, "mouse_click", mx, my, number);
            }
        }

        for (mouse_button, number) in MOUSE_BUTTON_NUMBERS {
            if rl.is_mouse_button_released(mouse_button) {
                send_mouse_event(kernel, "mouse_up", mx, my, number);
            }
        }

        let mouse_wheel = rl.get_mouse_wheel_move_v().y as i32;

        if mouse_wheel != 0 {
            kernel.borrow_static().send_event_to_root(
                "mouse_scroll",
                &ByteBuilder::new().u16(mx).u16(my).i32(mouse_wheel).build(),
            );
        }
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
