// Note: this driver is actually intended for all Wasm-os distributions, Provided they use 8-bit
// pixel graphics

use crate::kernel::ProcessLinker;

pub fn load_graphics_functions<T>(linker: &mut ProcessLinker<T>) -> wasmtime::Result<()> {
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

    pub fn draw_pixel(&mut self, memory: *mut u8, x: i32, y: i32, pixel: u8) {
        let (x, y) = self.camera.translate(x, y);
        self.draw_region.set_pixel(memory, x, y, pixel);
    }

    pub fn draw_pixel_checked(&mut self, memory: *mut u8, x: i32, y: i32, pixel: u8) {
        if pixel != self.transparency_color {
            self.draw_pixel(memory, x, y, pixel);
        }
    }

    pub fn draw_line(&mut self, memory: *mut u8, x1: i32, y1: i32, x2: i32, y2: i32, color: u8) {
        todo!()
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
        todo!()
    }

    pub fn draw_rectangle(
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

    pub fn draw_filled_rectangle(
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

    pub fn draw_round_rectangle(
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

    pub fn draw_circle(&mut self, memory: *mut u8, x: i32, y: i32, radius: i32, color: u8) {
        todo!()
    }

    pub fn draw_filled_circle(&mut self, memory: *mut u8, x: i32, y: i32, radius: i32, color: u8) {
        todo!()
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

struct DrawRegion {
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

    fn get_offset(&self, x: i32, y: i32) -> Option<isize> {
        let (x, y) = (x as isize, y as isize);
        let (width, height) = (self.width as isize, self.height as isize);

        if x < 0 || x >= width || y < 0 || y >= height {
            None
        } else {
            Some(x + y * width)
        }
    }

    /// Draws a pixel to memory if it is inside the bounds of the drawing region.
    /// Note: the memory pointer must start at the drawing region.
    pub fn set_pixel(&mut self, memory: *mut u8, x: i32, y: i32, pixel: u8) {
        if let Some(offset) = self.get_offset(x, y) {
            unsafe {
                memory.offset(offset).write(pixel);
            }
        }
    }

    pub unsafe fn get_pixel_unchecked(&mut self, memory: *const u8, x: i32, y: i32) -> u8 {
        unsafe { memory.offset((x + y * self.width as i32) as isize).read() }
    }
}

struct Camera {
    pub x: i32,
    pub y: i32,
}

impl Camera {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn translate(&self, x: i32, y: i32) -> (i32, i32) {
        (x - self.x, y - self.y)
    }
}
