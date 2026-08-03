use core::slice;

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

    /// Gets the pixel at the given position, with that position being wrapped to within the texture.
    pub fn get_pixel_wrapped(&self, memory: *const u8, x: i32, y: i32) -> u8 {
        let (x, y) = (x % self.width as i32, y % self.height as i32);
        unsafe { memory.add(self.as_index(x, y)).read() }
    }

    /// Clamps the width and x position to be inside this region.
    /// Returns x, width, and index offset if x is less than 0.
    pub fn clamp_width(&self, mut x: i32, mut width: u32) -> (i32, u32, u32) {
        let mut offset = 0;
        if x < 0 {
            offset = x.unsigned_abs();
            width += offset;
            x = 0;
        }

        width = width.min(self.width.saturating_sub(x as u32));

        (x, width, offset)
    }

    /// Clamps the height and y position to be inside this region.
    /// Returns y, height, and index offset if y is less than 0.
    pub fn clamp_height(&self, mut y: i32, mut height: u32) -> (i32, u32, u32) {
        let mut offset = 0;
        if y < 0 {
            offset = y.unsigned_abs();
            height += offset;
            y = 0;
        }

        height = height.min(self.height.saturating_sub(y as u32));

        (y, height, offset)
    }

    pub fn get_line(&self, memory: *const u8, i: usize) -> &[u8] {
        let offset = i * self.width as usize;
        unsafe { slice::from_raw_parts(memory.add(offset), self.width as usize) }
    }
}
