use std::num::NonZeroU32;

use crate::graphics::{
    camera::Camera,
    color::{self, Color},
    draw_region::DrawRegion,
};

pub const FONT_SIZE: usize = 8 * 256;
// TODO: create a default font file using some kind of bitmap drawing program.
pub const DEFAULT_FONT: [u8; FONT_SIZE] = [0; FONT_SIZE];

/// Represents the state of drawing within a process.
/// The reason this is part of the process itself is to avoid the overhead of type conversion within
/// drivers.
pub struct GraphicsState {
    /// The current transparency color
    pub transparency_color: u8,
    /// The 2d region of memory to draw pixels to.
    pub draw_region: DrawRegion,
    /// The address to draw to.
    pub draw_address: usize,
    /// The camera.
    pub camera: Camera,
    /// The optional address of the bitmap font to use in `draw_text`.
    /// When None, the default font is used.
    pub font_address: Option<NonZeroU32>,
    pub fill_pattern: [u8; 8],
}

impl GraphicsState {
    pub fn new() -> Self {
        Self {
            transparency_color: 0,
            draw_region: DrawRegion::new(0, 0),
            draw_address: 0,
            camera: Camera::new(0, 0),
            font_address: None,
            fill_pattern: [0; 8],
        }
    }

    pub fn set_font(&mut self, font_address: u32) {
        self.font_address = NonZeroU32::new(font_address);
    }

    pub fn font_address(&self) -> Option<NonZeroU32> {
        self.font_address
    }

    pub fn use_default_font(&mut self) {
        self.font_address = None;
    }

    pub fn set_fill_pattern(&mut self, fillp: u64) {
        self.fill_pattern = fillp.to_be_bytes();
    }

    pub fn get_fill_pattern(&self) -> u64 {
        u64::from_be_bytes(self.fill_pattern)
    }

    /// Gets the fill pattern line for any y coordinate in the draw region.
    fn get_fill_pattern_line(&self, y: usize, color: Color) -> [u8; 8] {
        let (fg, bg) = color::split_color(color);
        byte_to_8_bytes(self.fill_pattern[y % 8], bg, fg)
    }

    fn get_fill_pattern_pixel(&self, x: usize, y: usize, color: Color) -> u8 {
        self.get_fill_pattern_line(y, color)[x % 8]
    }

    pub fn draw_pixel_untranslated(&mut self, memory: *mut u8, x: i32, y: i32, color: Color) {
        let color = self.get_fill_pattern_pixel(x as usize, y as usize, color);
        self.draw_region.set_pixel(memory, x, y, color);
    }

    pub fn draw_pixel(&mut self, memory: *mut u8, x: i32, y: i32, color: Color) {
        let (x, y) = self.camera.translate(x, y);
        self.draw_pixel_untranslated(memory, x, y, color);
    }

    // Checks is the pixel is the transparency color.
    pub fn draw_pixel_checked(&mut self, memory: *mut u8, x: i32, y: i32, color: Color) {
        if color as u8 != self.transparency_color {
            self.draw_pixel(memory, x, y, color);
        }
    }

    // Uses Bresenham's algorithm to iterate the points on a line
    fn line_points(
        &mut self,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        mut action: impl FnMut(&mut Self, i32, i32),
    ) {
        let (x1, y1) = self.camera.translate(x1, y1);
        let (x2, y2) = self.camera.translate(x2, y2);

        let m_new = (y2 - y1) * 2;
        let mut slope_error_new = m_new - (x2 - x1);

        let mut y = y1;
        for x in x1..(x2 + 1) {
            action(self, x, y);

            slope_error_new += m_new;

            if slope_error_new >= 0 {
                y += 1;
                slope_error_new -= (y2 - y1) * 2;
            }
        }
    }

    pub fn draw_line(&mut self, memory: *mut u8, x1: i32, y1: i32, x2: i32, y2: i32, color: Color) {
        self.line_points(x1, y1, x2, y2, |graphics_state, x, y| {
            graphics_state.draw_pixel(memory, x, y, color);
        });
    }

    pub fn draw_textured_line(
        &mut self,
        memory: *mut u8,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        texture: *const u8,
        tex_width: u32,
        tex_height: u32,
        mut tex_x: f32,
        mut tex_y: f32,
        tex_dx: f32,
        tex_dy: f32,
    ) {
        let texture_region = DrawRegion::new(tex_width, tex_height);

        self.line_points(x1, y1, x2, y2, |graphics_state, x, y| {
            graphics_state.draw_pixel(
                memory,
                x,
                y,
                // TODO: allow fill patterns to look up colors in a secondary palette
                texture_region.get_pixel_wrapped(texture, tex_x as i32, tex_y as i32) as Color,
            );
            tex_x += tex_dx;
            tex_y += tex_dy;
        });
    }

    pub fn draw_hline(&mut self, memory: *mut u8, x: i32, y: i32, width: u32, color: Color) {
        let (x, y) = self.camera.translate(x, y);

        if !self.draw_region.inside_height(y) {
            return;
        }

        let (x, width, _) = self.draw_region.clamp_width(x, width);

        let index = self.draw_region.as_index(x, y);
        unsafe {
            // SAFETY: x is positive and if it goes out of bounds, then width is 0.
            let destination = memory.add(index);
            for i in 0..width as usize {
                let pixel = self.get_fill_pattern_pixel(x as usize + i, y as usize, color);
                destination.add(i).write(pixel);
            }
        }
    }

    pub fn draw_vline(&mut self, memory: *mut u8, x: i32, y: i32, height: u32, color: Color) {
        let (x, y) = self.camera.translate(x, y);

        if !self.draw_region.inside_height(y) {
            return;
        }

        let (y, height, _) = self.draw_region.clamp_height(y, height);

        for i in 0..height {
            let y = y + i as i32;
            let color = self.get_fill_pattern_pixel(x as usize, y as usize, color);
            unsafe {
                self.draw_region.set_pixel_unchecked(memory, x, y, color);
            }
        }
    }
    pub fn draw_rectangle(
        &mut self,
        memory: *mut u8,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        color: Color,
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
        color: Color,
    ) {
        let (_, ty) = self.camera.translate(x, y);
        let (_, height, offset) = self.draw_region.clamp_height(ty, height);

        for i in offset..height {
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
        color: Color,
    ) {
        // Clamp radius to prevent visual glitches
        let radius = radius.min(width.min(height) / 2) as i32;
        let inner_width = width.saturating_sub(2 * radius as u32);
        let inner_height = height.saturating_sub(2 * radius as u32);

        self.draw_circle_octant_points(radius as u32, |graphics_state, point_x, point_y| {
            let (point_x, point_y) = graphics_state.camera.translate(point_x, point_y);

            // Top-left
            let (cx, cy) = (x + radius, y + radius);
            graphics_state.draw_pixel_untranslated(memory, cx - point_x, cy + point_y, color);
            graphics_state.draw_pixel_untranslated(memory, cx + point_y, cy - point_x, color);

            // Top-right
            let (cx, cy) = (x + inner_width as i32 + radius, y + radius);
            graphics_state.draw_pixel_untranslated(memory, cx + point_x, cy + point_y, color);
            graphics_state.draw_pixel_untranslated(memory, cx - point_y, cy - point_x, color);

            // Bottom-left
            let (cx, cy) = (x + radius, y + inner_height as i32 + radius);
            graphics_state.draw_pixel_untranslated(memory, cx - point_x, cy - point_y, color);
            graphics_state.draw_pixel_untranslated(memory, cx + point_y, cy + point_x, color);

            // Bottom-right
            let (cx, cy) = (
                x + inner_width as i32 + radius,
                y + inner_height as i32 + radius,
            );
            graphics_state.draw_pixel_untranslated(memory, cx + point_x, cy - point_y, color);
            graphics_state.draw_pixel_untranslated(memory, cx - point_y, cy + point_x, color);
        });

        // Draw connecting lines
        self.draw_hline(memory, x + radius, y, inner_width, color);
        self.draw_hline(
            memory,
            x + radius,
            y + inner_height as i32 + radius * 2,
            inner_width,
            color,
        );
        self.draw_vline(memory, x, y + radius, inner_height, color);
        self.draw_vline(
            memory,
            x + inner_width as i32 + radius * 2,
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
        color: Color,
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

        self.draw_filled_rectangle(memory, x, y + radius, width, inner_height, color);
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

    pub fn draw_circle(&mut self, memory: *mut u8, cx: i32, cy: i32, radius: u32, color: Color) {
        self.draw_circle_octant_points(radius, |graphics_state, x, y| {
            let (cx, cy) = graphics_state.camera.translate(cx, cy);

            graphics_state.draw_pixel_untranslated(memory, cx + x, cy + y, color);
            graphics_state.draw_pixel_untranslated(memory, cx - x, cy + y, color);
            graphics_state.draw_pixel_untranslated(memory, cx + x, cy - y, color);
            graphics_state.draw_pixel_untranslated(memory, cx - x, cy - y, color);
            graphics_state.draw_pixel_untranslated(memory, cx + y, cy + x, color);
            graphics_state.draw_pixel_untranslated(memory, cx - y, cy + x, color);
            graphics_state.draw_pixel_untranslated(memory, cx + y, cy - x, color);
            graphics_state.draw_pixel_untranslated(memory, cx - y, cy - x, color);
        })
    }

    pub fn draw_filled_circle(
        &mut self,
        memory: *mut u8,
        cx: i32,
        cy: i32,
        radius: u32,
        color: Color,
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
        color: Color,
        action: fn(&mut Self, *mut u8, i32, i32, i32, i32, Color),
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
        color: Color,
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
                graphics_state.draw_pixel_untranslated(memory, cx + x, cy + y, color);
                graphics_state.draw_pixel_untranslated(memory, cx - x, cy + y, color);
                graphics_state.draw_pixel_untranslated(memory, cx + x, cy - y, color);
                graphics_state.draw_pixel_untranslated(memory, cx - x, cy - y, color);
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
        color: Color,
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
        mut y: i32,
        sprite: *const u8,
        spr_width: u32,
        mut spr_height: u32,
    ) {
        let sprite_region = DrawRegion::new(spr_width, spr_height);

        let mut line_y = 0;
        if y < 0 {
            line_y = y.unsigned_abs();
            spr_height = spr_height.saturating_sub(line_y);
            y = 0;
        }

        for i in 0..spr_height {
            let line = sprite_region.get_line(sprite, (line_y + i) as usize);
            self.draw_line_bytes(memory, x, y + i as i32, line);
        }
    }

    /// Draws a sequence of bytes as a line.
    fn draw_line_bytes(&mut self, memory: *mut u8, x: i32, y: i32, line: &[u8]) {
        let (x, y) = self.camera.translate(x, y);

        if !self.draw_region.inside_height(y) {
            return;
        }

        self.draw_line_bytes_untranslated(memory, x, y, line);
    }

    fn draw_line_bytes_untranslated(&mut self, memory: *mut u8, x: i32, y: i32, line: &[u8]) {
        let (x, width, offset) = self.draw_region.clamp_width(x, line.len() as u32);

        // SAFETY: the line's length is guarenteed to be within the drawing region
        unsafe {
            let line_destination = memory.add(self.draw_region.as_index(x, y));
            for i in 0..width as usize {
                let pixel = *line.get_unchecked(i + offset as usize);
                if pixel != self.transparency_color {
                    line_destination.add(i).write(pixel);
                }
            }
        }
    }

    pub fn draw_map(
        &mut self,
        _memory: *mut u8,
        _map: *const u8,
        _map_width: usize,
        _map_height: usize,
        _spritesheet: *const u8,
        _spr_width: u32,
        _spr_height: u32,
    ) {
        // TODO: do much later
        todo!()
    }

    pub fn draw_text(
        &mut self,
        memory: *mut u8,
        font: &[u8; FONT_SIZE],
        text: &[u8],
        x: i32,
        y: i32,
        fg: u8,
        bg: u8,
    ) {
        let (mut x, mut y) = self.camera.translate(x, y);
        let start_x = x;

        for &character in text {
            if !self.draw_region.inside_height(y) {
                break;
            }

            if !self.draw_region.inside_width(x) || character == b'\n' {
                x = start_x;
                y += 8;
            } else {
                self.draw_character_untranslated(memory, font, character, x, y, fg, bg);
                x += 8;
            }
        }
    }

    /// Draws a character to the exact screen coordinates
    fn draw_character_untranslated(
        &mut self,
        memory: *mut u8,
        font: &[u8; FONT_SIZE],
        character: u8,
        x: i32,
        y: i32,
        fg: u8,
        bg: u8,
    ) {
        let character = character as usize;
        for i in 0..8 {
            let line = byte_to_8_bytes(font[character * 8 + i], fg, bg);
            self.draw_line_bytes_untranslated(memory, x, y + i as i32, &line);
        }
    }
}

fn byte_to_8_bytes(byte: u8, one_value: u8, zero_value: u8) -> [u8; 8] {
    let mut bytes = [0; 8];

    for (i, out_byte) in bytes.iter_mut().enumerate() {
        let bit = (byte >> i) & 1;
        *out_byte = if bit == 1 { one_value } else { zero_value }
    }

    bytes
}
