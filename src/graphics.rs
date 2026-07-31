// Note: this driver is actually intended for all Wasm-os distributions, Provided they use 8-bit
// pixel graphics

use crate::kernel::{ProcessContext, ProcessLinker};

pub fn load_graphics_functions<T>(linker: &mut ProcessLinker<T>) -> wasmtime::Result<()> {
    linker.func_wrap(
        "env",
        "set_draw_region",
        |mut ctx: ProcessContext<T>, address: u32, width: u32, height: u32| {
            ctx.data_mut().graphics_state.draw_region = DrawRegion::new(width, height);
            ctx.data_mut().graphics_state.draw_address = address as usize;
        },
    )?;

    linker.func_wrap(
        "env",
        "set_draw_region",
        |mut ctx: ProcessContext<T>, address: i32, width: u32, height: u32| {
            ctx.data_mut().graphics_state.draw_region = DrawRegion::new(width, height);
            ctx.data_mut().graphics_state.draw_address = address as usize;
        },
    )?;

    linker.func_wrap(
        "env",
        "set_transparency_color",
        |mut ctx: ProcessContext<T>, color: i32| -> i32 {
            let old_color = ctx.data_mut().graphics_state.transparency_color;
            ctx.data_mut().graphics_state.transparency_color = color as u8;
            old_color as i32
        },
    )?;

    linker.func_wrap(
        "env",
        "set_camera",
        |mut ctx: ProcessContext<T>, x: i32, y: i32| {
            ctx.data_mut().graphics_state.camera.set_position(x, y);
        },
    )?;

    linker.func_wrap("env", "get_camera_x", |mut ctx: ProcessContext<T>| -> i32 {
        ctx.data_mut().graphics_state.camera.x
    })?;

    linker.func_wrap("env", "get_camera_y", |mut ctx: ProcessContext<T>| -> i32 {
        ctx.data_mut().graphics_state.camera.y
    })?;

    linker.func_wrap(
        "env",
        "draw_pixel",
        |mut ctx: ProcessContext<T>, x: i32, y: i32, pixel: i32| {
            let draw_address = ctx.data().get_draw_address();
            ctx.data_mut()
                .graphics_state
                .draw_pixel_checked(draw_address, x, y, pixel as u8);
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_line",
        |mut ctx: ProcessContext<T>, x1: i32, y1: i32, x2: i32, y2: i32, color: i32| {
            let draw_address = ctx.data().get_draw_address();
            ctx.data_mut()
                .graphics_state
                .draw_line(draw_address, x1, y1, x2, y2, color as u8);
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_hline",
        |mut ctx: ProcessContext<T>, x: i32, y: i32, width: u32, color: i32| {
            let draw_address = ctx.data().get_draw_address();
            ctx.data_mut()
                .graphics_state
                .draw_hline(draw_address, x, y, width, color as u8);
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_vline",
        |mut ctx: ProcessContext<T>, x: i32, y: i32, height: u32, color: i32| {
            let draw_address = ctx.data().get_draw_address();
            ctx.data_mut()
                .graphics_state
                .draw_vline(draw_address, x, y, height, color as u8);
        },
    )?;
    todo!()
}

const FONT_SIZE: usize = 8 * 256;

// TODO: create a default font file using some kind of bitmap drawing program.
const DEFAULT_FONT: [u8; FONT_SIZE] = [0; FONT_SIZE];

pub struct GraphicsState {
    pub transparency_color: u8,
    pub draw_region: DrawRegion,
    pub draw_address: usize,
    pub camera: Camera,
    pub font: [u8; FONT_SIZE],
}

impl GraphicsState {
    pub fn new() -> Self {
        Self {
            transparency_color: 0,
            draw_region: DrawRegion::new(0, 0),
            draw_address: 0,
            camera: Camera::new(0, 0),
            font: DEFAULT_FONT,
        }
    }

    pub fn set_font(&mut self, font_memory: *const u8) {
        // SAFETY: assumes that the program's font does not go outside the bounts of its memory
        unsafe {
            self.font = font_memory.cast::<[u8; FONT_SIZE]>().read();
        }
    }

    pub fn use_default_font(&mut self) {
        self.font = DEFAULT_FONT;
    }

    pub fn draw_pixel(&mut self, memory: *mut u8, x: i32, y: i32, color: u8) {
        let (x, y) = self.camera.translate(x, y);
        self.draw_region.set_pixel(memory, x, y, color);
    }

    // Checks is the pixel is the transparency color.
    pub fn draw_pixel_checked(&mut self, memory: *mut u8, x: i32, y: i32, color: u8) {
        if color != self.transparency_color {
            self.draw_pixel(memory, x, y, color);
        }
    }

    // Uses Bresenham's algorithm to quickly draw a line.
    pub fn draw_line(&mut self, memory: *mut u8, x1: i32, y1: i32, x2: i32, y2: i32, color: u8) {
        let (x1, y1) = self.camera.translate(x1, y1);
        let (x2, y2) = self.camera.translate(x2, y2);

        let m_new = (y2 - y1) * 2;
        let mut slope_error_new = m_new - (x2 - x1);

        let mut y = y1;
        for x in x1..(x2 + 1) {
            self.draw_region.set_pixel(memory, x, y, color);

            slope_error_new += m_new;

            if slope_error_new >= 0 {
                y += 1;
                slope_error_new -= (y2 - y1) * 2;
            }
        }
    }

    pub fn draw_hline(&mut self, memory: *mut u8, x: i32, y: i32, width: u32, color: u8) {
        let (x, y) = self.camera.translate(x, y);

        if !self.draw_region.inside_height(y) {
            return;
        }

        let (x, width) = self.draw_region.clamp_width(x, width);

        for i in 0..width {
            unsafe {
                self.draw_region
                    .set_pixel_unchecked(memory, x + i as i32, y, color);
            }
        }
    }

    pub fn draw_vline(&mut self, memory: *mut u8, x: i32, y: i32, height: u32, color: u8) {
        let (x, y) = self.camera.translate(x, y);

        if !self.draw_region.inside_height(y) {
            return;
        }

        let (y, height) = self.draw_region.clamp_height(y, height);

        for i in 0..height {
            unsafe {
                self.draw_region
                    .set_pixel_unchecked(memory, x, y + i as i32, color);
            }
        }
    }

    pub fn draw_textured_line(
        &mut self,
        memory: *mut u8,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        texture: *const u8,
        tex_width: i32,
        tex_height: i32,
        tex_x: i32,
        tex_y: i32,
        tex_dx: i32,
        tex_dy: i32,
    ) {
        // TODO: do this a LOT later
        todo!()
    }

    pub fn draw_rectangle(
        &mut self,
        memory: *mut u8,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        color: u8,
    ) {
        self.draw_hline(memory, x, y, width, color);
        self.draw_vline(memory, x, y, height, color);
        self.draw_hline(memory, x, y + height.saturating_sub(1) as i32, width, color);
        self.draw_vline(memory, x + width.saturating_sub(1) as i32, y, height, color);
    }

    pub fn draw_filled_rectangle(
        &mut self,
        memory: *mut u8,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        color: u8,
    ) {
        let (_, ty) = self.camera.translate(x, y);
        let (_, height) = self.draw_region.clamp_height(ty, height);

        for i in 0..height {
            self.draw_hline(memory, x, y + i as i32, width, color);
        }
    }

    pub fn draw_round_rectangle(
        &mut self,
        memory: *mut u8,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        color: u8,
    ) {
        // TODO: do MUCH later
        todo!()
    }

    pub fn draw_filled_round_rectangle(
        &mut self,
        memory: *mut u8,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        color: u8,
    ) {
        todo!()
    }

    pub fn draw_circle(&mut self, memory: *mut u8, cx: i32, cy: i32, radius: i32, color: u8) {
        let (cx, cy) = self.camera.translate(cx, cy);
        let mut x = 0;
        let mut y = -radius;
        let mut p = -radius;

        while x < -y {
            if p > 0 {
                y += 1;
                p += 2 * (x + y) + 1;
            } else {
                p += 2 * x + 1
            }

            self.draw_region.set_pixel(memory, cx + x, cy + y, color);
            self.draw_region.set_pixel(memory, cx - x, cy + y, color);
            self.draw_region.set_pixel(memory, cx + x, cy - y, color);
            self.draw_region.set_pixel(memory, cx - x, cy - y, color);
            self.draw_region.set_pixel(memory, cx + y, cy + x, color);
            self.draw_region.set_pixel(memory, cx - y, cy + x, color);
            self.draw_region.set_pixel(memory, cx + y, cy - x, color);
            self.draw_region.set_pixel(memory, cx - y, cy - x, color);

            x += 1;
        }
    }

    pub fn draw_filled_circle(
        &mut self,
        memory: *mut u8,
        cx: i32,
        cy: i32,
        radius: i32,
        color: u8,
    ) {
        // No need for transform, it's done by draw_hline
        let mut x = 0;
        let mut y = -radius;
        let mut p = -radius;

        while x < -y {
            if p > 0 {
                y += 1;
                p += 2 * (x + y) + 1;
            } else {
                p += 2 * x + 1
            }

            // TODO: check that the width argument is correct
            let px = cx - x;
            let width = (cx + x - px) as u32;
            self.draw_hline(memory, px, cy + y, width, color);
            self.draw_hline(memory, px, cy - y, width, color);

            let px = cx - y;
            let width = (cx + y - px) as u32;
            self.draw_hline(memory, px, cy + x, width, color);
            self.draw_hline(memory, px, cy - x, width, color);

            x += 1;
        }
    }

    pub fn draw_ellipse(
        &mut self,
        memory: *mut u8,
        x: i32,
        y: i32,
        h_radius: i32,
        v_radius: i32,
        color: u8,
    ) {
        todo!()
    }

    pub fn draw_filled_ellipse(
        &mut self,
        memory: *mut u8,
        x: i32,
        y: i32,
        h_radius: i32,
        v_radius: i32,
        color: u8,
    ) {
        todo!()
    }

    pub fn draw_sprite(
        &mut self,
        memory: *mut u8,
        x: i32,
        y: i32,
        sprite: *const u8,
        spr_width: i32,
        spr_height: i32,
    ) {
        todo!()
    }

    pub fn draw_map(
        &mut self,
        memory: *mut u8,
        map: *const u8,
        map_width: usize,
        map_height: usize,
        spritesheet: *const u8,
        spr_width: usize,
        spr_height: usize,
    ) {
        todo!()
    }

    pub fn draw_text(&mut self, memory: *mut u8, text: &[u8], fg: u8, bg: u8) {
        todo!()
    }
}

pub struct DrawRegion {
    pub width: u32,
    pub height: u32,
}

impl DrawRegion {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    fn area(&self) -> u32 {
        self.width * self.height
    }

    pub fn inside_width(&self, x: i32) -> bool {
        x >= 0 && x < self.width as i32
    }

    pub fn inside_height(&self, y: i32) -> bool {
        y >= 0 && y < self.height as i32
    }

    pub unsafe fn set_pixel_unchecked(&mut self, memory: *mut u8, x: i32, y: i32, pixel: u8) {
        let offset = (x + y * self.width as i32) as isize;
        unsafe {
            memory.offset(offset).write(pixel);
        }
    }

    /// Draws a pixel to memory if it is inside the bounds of the drawing region.
    /// Note: the memory pointer must start at the drawing region.
    pub fn set_pixel(&mut self, memory: *mut u8, x: i32, y: i32, pixel: u8) {
        if self.inside_width(x) && self.inside_height(y) {
            unsafe {
                self.set_pixel_unchecked(memory, x, y, pixel);
            }
        }
    }

    /// Clamps the width and x position to be inside this region.
    pub fn clamp_width(&self, mut x: i32, mut width: u32) -> (i32, u32) {
        if x < 0 {
            width += x.abs() as u32;
            x = 0;
        }

        width = width.min(self.width);

        (x, width)
    }

    /// Clamps the height and y position to be inside this region.
    pub fn clamp_height(&self, mut y: i32, mut height: u32) -> (i32, u32) {
        if y < 0 {
            height += y.abs() as u32;
            y = 0;
        }

        height = height.min(self.height);

        (y, height)
    }
}

pub struct Camera {
    pub x: i32,
    pub y: i32,
}

impl Camera {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn set_position(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    pub fn translate(&self, x: i32, y: i32) -> (i32, i32) {
        (x - self.x, y - self.y)
    }
}
