pub struct ByteBuilder {
    buffer: Vec<u8>,
}

impl ByteBuilder {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(64),
        }
    }

    pub fn build(self) -> Vec<u8> {
        self.buffer
    }

    pub fn u8(mut self, value: u8) -> Self {
        self.buffer.push(value);
        self
    }

    pub fn u16(mut self, value: u16) -> Self {
        self.buffer.extend_from_slice(&value.to_le_bytes());
        self
    }

    pub fn i32(mut self, value: i32) -> Self {
        self.buffer.extend_from_slice(&value.to_le_bytes());
        self
    }
}
