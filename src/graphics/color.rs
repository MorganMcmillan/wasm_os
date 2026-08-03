pub type Color = u16;

pub fn split_color(color: Color) -> (u8, u8) {
    (color as u8, (color >> 8) as u8)
}
