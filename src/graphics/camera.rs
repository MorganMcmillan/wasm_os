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
