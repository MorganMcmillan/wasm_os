#![allow(clippy::too_many_arguments)]

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
        "clear_draw_region",
        |ctx: ProcessContext<T>, color: u32| {
            let address = ctx.data().get_draw_address();
            unsafe {
                address.write_bytes(
                    color as u8,
                    ctx.data().graphics_state.draw_region.area() as usize,
                );
            }
        },
    )?;

    linker.func_wrap(
        "env",
        "set_transparency_color",
        |mut ctx: ProcessContext<T>, color: u32| -> u32 {
            let old_color = ctx.data_mut().graphics_state.transparency_color;
            ctx.data_mut().graphics_state.transparency_color = color as u8;
            old_color as u32
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
        |mut ctx: ProcessContext<T>, x: i32, y: i32, pixel: u32| {
            let draw_address = ctx.data().get_draw_address();
            ctx.data_mut()
                .graphics_state
                .draw_pixel_checked(draw_address, x, y, pixel as u8);
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_line",
        |mut ctx: ProcessContext<T>, x1: i32, y1: i32, x2: i32, y2: i32, color: u32| {
            let draw_address = ctx.data().get_draw_address();
            ctx.data_mut()
                .graphics_state
                .draw_line(draw_address, x1, y1, x2, y2, color as u8);
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_hline",
        |mut ctx: ProcessContext<T>, x: i32, y: i32, width: u32, color: u32| {
            let draw_address = ctx.data().get_draw_address();
            ctx.data_mut()
                .graphics_state
                .draw_hline(draw_address, x, y, width, color as u8);
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_vline",
        |mut ctx: ProcessContext<T>, x: i32, y: i32, height: u32, color: u32| {
            let draw_address = ctx.data().get_draw_address();
            ctx.data_mut()
                .graphics_state
                .draw_vline(draw_address, x, y, height, color as u8);
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_rectangle",
        |mut ctx: ProcessContext<T>, x: i32, y: i32, width: u32, height: u32, color: u32| {
            let draw_address = ctx.data().get_draw_address();
            ctx.data_mut().graphics_state.draw_rectangle(
                draw_address,
                x,
                y,
                width,
                height,
                color as u8,
            );
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_filled_rectangle",
        |mut ctx: ProcessContext<T>, x: i32, y: i32, width: u32, height: u32, color: u32| {
            let draw_address = ctx.data().get_draw_address();
            ctx.data_mut().graphics_state.draw_filled_rectangle(
                draw_address,
                x,
                y,
                width,
                height,
                color as u8,
            );
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_round_rectangle",
        |mut ctx: ProcessContext<T>,
         x: i32,
         y: i32,
         width: u32,
         height: u32,
         radius: u32,
         color: u32| {
            let draw_address = ctx.data().get_draw_address();
            ctx.data_mut().graphics_state.draw_round_rectangle(
                draw_address,
                x,
                y,
                width,
                height,
                radius,
                color as u8,
            );
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_filled_round_rectangle",
        |mut ctx: ProcessContext<T>,
         x: i32,
         y: i32,
         width: u32,
         height: u32,
         radius: u32,
         color: u32| {
            let draw_address = ctx.data().get_draw_address();
            ctx.data_mut().graphics_state.draw_filled_round_rectangle(
                draw_address,
                x,
                y,
                width,
                height,
                radius,
                color as u8,
            );
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_circle",
        |mut ctx: ProcessContext<T>, x: i32, y: i32, radius: u32, color: u32| {
            let draw_address = ctx.data().get_draw_address();
            ctx.data_mut()
                .graphics_state
                .draw_circle(draw_address, x, y, radius, color as u8);
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_filled_circle",
        |mut ctx: ProcessContext<T>, x: i32, y: i32, radius: u32, color: u32| {
            let draw_address = ctx.data().get_draw_address();
            ctx.data_mut().graphics_state.draw_filled_circle(
                draw_address,
                x,
                y,
                radius,
                color as u8,
            );
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_ellipse",
        |mut ctx: ProcessContext<T>, x: i32, y: i32, x_radius: u32, y_radius: u32, color: u32| {
            let draw_address = ctx.data().get_draw_address();
            ctx.data_mut().graphics_state.draw_ellipse(
                draw_address,
                x,
                y,
                x_radius,
                y_radius,
                color as u8,
            );
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_filled_ellipse",
        |mut ctx: ProcessContext<T>, x: i32, y: i32, x_radius: u32, y_radius: u32, color: u32| {
            let draw_address = ctx.data().get_draw_address();
            ctx.data_mut().graphics_state.draw_filled_ellipse(
                draw_address,
                x,
                y,
                x_radius,
                y_radius,
                color as u8,
            );
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_sprite",
        |mut ctx: ProcessContext<T>,
         x: i32,
         y: i32,
         sprite: i32,
         spr_width: u32,
         spr_height: u32| {
            let draw_address = ctx.data().get_draw_address();

            let spr_width = spr_width as usize;
            let spr_height = spr_height as usize;
            let sprite = ctx
                .data()
                .get_memory_mut(sprite as usize, spr_width * spr_height)
                .as_mut_ptr();

            ctx.data_mut().graphics_state.draw_sprite(
                draw_address,
                x,
                y,
                sprite,
                spr_width,
                spr_height,
            );
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_map",
        |mut ctx: ProcessContext<T>,
         map: i32,
         map_width: u32,
         map_height: u32,
         spritesheet: i32,
         spr_width: u32,
         spr_height: u32| {
            let draw_address = ctx.data().get_draw_address();

            let map_width = map_width as usize;
            let map_height = map_height as usize;
            let map = ctx
                .data()
                .get_memory_mut(map as usize, map_width * map_height)
                .as_mut_ptr();

            let spr_width = spr_width as usize;
            let spr_height = spr_height as usize;
            let spritesheet = ctx
                .data()
                .get_memory_mut(spritesheet as usize, spr_width * spr_height * 256)
                .as_mut_ptr();

            ctx.data_mut().graphics_state.draw_map(
                draw_address,
                map,
                map_width,
                map_height,
                spritesheet,
                spr_width,
                spr_height,
            );
        },
    )?;

    linker.func_wrap(
        "env",
        "set_font",
        |mut ctx: ProcessContext<T>, font_ptr: i32| {
            let font = ctx
                .data()
                .get_memory_mut(font_ptr as usize, 256 * 8)
                .as_ptr();
            ctx.data_mut().graphics_state.set_font(font);
        },
    )?;

    linker.func_wrap("env", "use_default_font", |mut ctx: ProcessContext<T>| {
        ctx.data_mut().graphics_state.use_default_font();
    })?;

    linker.func_wrap(
        "env",
        "draw_text",
        |mut ctx: ProcessContext<T>, text_ptr: i32, text_len: u32, fg: u32, bg: u32| {
            let draw_address = ctx.data().get_draw_address();

            let text = ctx
                .data()
                .get_memory_mut(text_ptr as usize, text_len as usize);

            ctx.data_mut()
                .graphics_state
                .draw_text(draw_address, text, fg as u8, bg as u8);
        },
    )?;

    Ok(())
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

        let index = self.draw_region.as_index(x, y);
        // SAFETY: x is positive and if it goes out of bounds, then width is 0.
        unsafe {
            memory.add(index).write_bytes(color, width as usize);
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
        width: u32,
        height: u32,
        radius: u32,
        color: u8,
    ) {
        // Clamp radius to prevent visual glitches
        let radius = radius.min(width.min(height) / 2) as i32;
        let inner_width = width.saturating_sub(2 * radius as u32);
        let inner_height = height.saturating_sub(2 * radius as u32);

        self.draw_circle_octant_points(radius as u32, |graphics_state, point_x, point_y| {
            let (point_x, point_y) = graphics_state.camera.translate(point_x, point_y);

            // Top-left
            let (cx, cy) = (x + radius, y + radius);
            graphics_state
                .draw_region
                .set_pixel(memory, cx - point_x, cy + point_y, color);
            graphics_state
                .draw_region
                .set_pixel(memory, cx + point_y, cy - point_x, color);

            // Top-right
            let (cx, cy) = (x + inner_width as i32 + radius, y + radius);
            graphics_state
                .draw_region
                .set_pixel(memory, cx + point_x, cy + point_y, color);
            graphics_state
                .draw_region
                .set_pixel(memory, cx - point_y, cy - point_x, color);

            // Bottom-left
            let (cx, cy) = (x + radius, y + inner_height as i32 + radius);
            graphics_state
                .draw_region
                .set_pixel(memory, cx - point_x, cy - point_y, color);
            graphics_state
                .draw_region
                .set_pixel(memory, cx + point_y, cy + point_x, color);

            // Bottom-right
            let (cx, cy) = (
                x + inner_width as i32 + radius,
                y + inner_height as i32 + radius,
            );
            graphics_state
                .draw_region
                .set_pixel(memory, cx + point_x, cy - point_y, color);
            graphics_state
                .draw_region
                .set_pixel(memory, cx - point_y, cy + point_x, color);
        });

        // Draw connecting lines
        self.draw_hline(memory, x + radius, y, inner_width, color);
        self.draw_hline(
            memory,
            x + radius,
            y + inner_height as i32 + radius,
            inner_width,
            color,
        );
        self.draw_vline(memory, x, y + radius, inner_height, color);
        self.draw_vline(
            memory,
            x + inner_width as i32 + radius,
            y + radius,
            inner_height,
            color,
        );
    }

    pub fn draw_filled_round_rectangle(
        &mut self,
        memory: *mut u8,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        radius: u32,
        color: u8,
    ) {
        // Clamp radius to prevent visual glitches
        let radius = radius.min(width.min(height) / 2) as i32;
        let inner_width = width.saturating_sub(2 * radius as u32);
        let inner_height = height.saturating_sub(2 * radius as u32);

        self.draw_circle_octant_points(radius as u32, |graphics_state, point_x, point_y| {
            let cx = x + radius;

            // Length 1
            let cy = y + radius;
            let px = cx - point_x;
            let line_width = (inner_width as i32 + 2 * point_x) as u32;
            graphics_state.draw_hline(memory, px, cy + point_y, line_width, color);
            let cy = y + inner_height as i32 + radius;
            graphics_state.draw_hline(memory, px, cy - point_y, line_width, color);

            // Length 2
            let cy = y + radius;
            let px = cx + point_y;
            let line_width = (inner_width as i32 - 2 * point_y) as u32;
            graphics_state.draw_hline(memory, px, cy - point_x, line_width, color);
            let cy = y + inner_height as i32 + radius;
            graphics_state.draw_hline(memory, px, cy + point_x, line_width, color);
        });

        self.draw_filled_rectangle(memory, x, y + radius, inner_width, inner_height, color);
    }

    /// Encapsulates the logic for drawing the pixels on a circle.
    /// Both x and y are for the top-top-right octant. Meaning: x is always positive, and y is
    /// always negative.
    fn draw_circle_octant_points(&mut self, radius: u32, action: impl Fn(&mut Self, i32, i32)) {
        let radius = radius as i32;
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

            action(self, x, y);

            x += 1;
        }
    }

    pub fn draw_circle(&mut self, memory: *mut u8, cx: i32, cy: i32, radius: u32, color: u8) {
        self.draw_circle_octant_points(radius, |graphics_state, x, y| {
            let (cx, cy) = graphics_state.camera.translate(cx, cy);

            graphics_state
                .draw_region
                .set_pixel(memory, cx + x, cy + y, color);
            graphics_state
                .draw_region
                .set_pixel(memory, cx - x, cy + y, color);
            graphics_state
                .draw_region
                .set_pixel(memory, cx + x, cy - y, color);
            graphics_state
                .draw_region
                .set_pixel(memory, cx - x, cy - y, color);
            graphics_state
                .draw_region
                .set_pixel(memory, cx + y, cy + x, color);
            graphics_state
                .draw_region
                .set_pixel(memory, cx - y, cy + x, color);
            graphics_state
                .draw_region
                .set_pixel(memory, cx + y, cy - x, color);
            graphics_state
                .draw_region
                .set_pixel(memory, cx - y, cy - x, color);
        })
    }

    pub fn draw_filled_circle(
        &mut self,
        memory: *mut u8,
        cx: i32,
        cy: i32,
        radius: u32,
        color: u8,
    ) {
        self.draw_circle_octant_points(radius, |graphics_state, x, y| {
            // No need for transform, it's done by draw_hline
            let px = cx - x;
            let width = (2 * x) as u32;
            graphics_state.draw_hline(memory, px, cy + y, width, color);
            graphics_state.draw_hline(memory, px, cy - y, width, color);

            let px = cx + y;
            let width = (2 * -y) as u32;
            graphics_state.draw_hline(memory, px, cy + x, width, color);
            graphics_state.draw_hline(memory, px, cy - x, width, color);
        })
    }

    fn draw_ellipse_quardrant_points(
        &mut self,
        memory: *mut u8,
        cx: i32,
        cy: i32,
        x_radius: i32,
        y_radius: i32,
        color: u8,
        action: fn(&mut Self, *mut u8, i32, i32, i32, i32, u8),
    ) {
        let mut x = 0;
        let mut y = y_radius;

        // Initial decision parameter of region 1
        let mut d1 = y_radius * y_radius - x_radius * x_radius * y_radius + x_radius * x_radius;
        let mut dx = 2 * y_radius * y_radius * x;
        let mut dy = 2 * x_radius * x_radius * y;

        // For region 1
        while dx < dy {
            action(self, memory, cx, cy, x, y, color);

            // Checking and updating value of
            // decision parameter based on algorithm
            if d1 < 0 {
                x += 1;
                dx += 2 * y_radius * y_radius;
                d1 += dx + y_radius * y_radius;
            } else {
                x += 1;
                y -= 1;
                dx += 2 * y_radius * y_radius;
                dy -= 2 * x_radius * x_radius;
                d1 += dx - dy + y_radius * y_radius;
            }
        }

        // Decision parameter of region 2
        let mut d2 = y_radius * y_radius * (x * x + x) + x_radius * x_radius * (y - 1) * (y - 1)
            - x_radius * x_radius * y_radius * y_radius;
        // Plotting points of region 2
        while y >= 0 {
            // printing points based on 4-way symmety_radius
            action(self, memory, cx, cy, x, y, color);

            // Checking and updating parameter
            // value based on algorithm
            if d2 > 0 {
                y -= 1;
                dy -= 2 * x_radius * x_radius;
                d2 += x_radius * x_radius - dy;
            } else {
                y -= 1;
                x += 1;
                dx += 2 * y_radius * y_radius;
                dy -= 2 * x_radius * x_radius;
                d2 += dx - dy + x_radius * x_radius;
            }
        }
    }

    pub fn draw_ellipse(
        &mut self,
        memory: *mut u8,
        cx: i32,
        cy: i32,
        x_radius: u32,
        y_radius: u32,
        color: u8,
    ) {
        self.draw_ellipse_quardrant_points(
            memory,
            cx,
            cy,
            x_radius as i32,
            y_radius as i32,
            color,
            |graphics_state, memory, cx, cy, x, y, color| {
                let (cx, cy) = graphics_state.camera.translate(cx, cy);
                graphics_state
                    .draw_region
                    .set_pixel(memory, cx + x, cy + y, color);
                graphics_state
                    .draw_region
                    .set_pixel(memory, cx - x, cy + y, color);
                graphics_state
                    .draw_region
                    .set_pixel(memory, cx + x, cy - y, color);
                graphics_state
                    .draw_region
                    .set_pixel(memory, cx - x, cy - y, color);
            },
        );
    }

    pub fn draw_filled_ellipse(
        &mut self,
        memory: *mut u8,
        cx: i32,
        cy: i32,
        x_radius: u32,
        y_radius: u32,
        color: u8,
    ) {
        self.draw_ellipse_quardrant_points(
            memory,
            cx,
            cy,
            x_radius as i32,
            y_radius as i32,
            color,
            |graphics_state, memory, cx, cy, x, y, color| {
                let px = cx - x;
                let width = (cx + x - px + 1) as u32;
                graphics_state.draw_hline(memory, px, cy + y, width, color);
                graphics_state.draw_hline(memory, px, cy - y, width, color);
            },
        );
    }

    pub fn draw_sprite(
        &mut self,
        memory: *mut u8,
        x: i32,
        y: i32,
        sprite: *const u8,
        spr_width: usize,
        spr_height: usize,
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

    pub fn area(&self) -> u32 {
        self.width * self.height
    }

    pub fn inside_width(&self, x: i32) -> bool {
        x >= 0 && x < self.width as i32
    }

    pub fn inside_height(&self, y: i32) -> bool {
        y >= 0 && y < self.height as i32
    }

    /// Converts x and y coordinates to a byte index, without checking that the are in bounds.
    pub fn as_index(&self, x: i32, y: i32) -> usize {
        (x + y * self.width as i32) as usize
    }

    pub unsafe fn set_pixel_unchecked(&mut self, memory: *mut u8, x: i32, y: i32, pixel: u8) {
        let index = self.as_index(x, y);
        unsafe {
            memory.add(index).write(pixel);
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
            width += x.unsigned_abs();
            x = 0;
        }

        width = width.min(self.width.saturating_sub(x as u32));

        (x, width)
    }

    /// Clamps the height and y position to be inside this region.
    pub fn clamp_height(&self, mut y: i32, mut height: u32) -> (i32, u32) {
        if y < 0 {
            height += y.unsigned_abs();
            y = 0;
        }

        height = height.min(self.height.saturating_sub(y as u32));

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
