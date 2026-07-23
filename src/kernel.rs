use crate::draw;
use crate::input;

pub struct Kernel {
    pub drawstate: draw::DrawState,
    pub mousestate: input::MouseState,
}

impl Kernel {
    pub fn new(drawstate: draw::DrawState) -> Self {
        Self {
            drawstate,
            mousestate: input::MouseState::new(),
        }
    }
}
