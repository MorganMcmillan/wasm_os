pub type Color = u16;
pub type Pixel = u8;

const RED_MASK: u8 = 0b11100000;
const GREEN_MASK: u8 = 0b00011100;
const BLUE_MASK: u8 = 0b00000011;

const RED_BIT_OFFSET: usize = 5;
const GREEN_BIT_OFFSET: usize = 2;

const RED_MAX: u8 = 7;
const GREEN_MAX: u8 = 7;
const BLUE_MAX: u8 = 3;

pub(crate) fn split_color(color: Color) -> (Pixel, Pixel) {
    (color as Pixel, (color >> (size_of::<Pixel>() * 8)) as Pixel)
}

fn pixel_to_rgb(pixel: Pixel) -> (u8, u8, u8) {
    let r: u8 = (pixel & RED_MASK) >> 5;
    let g: u8 = (pixel & GREEN_MASK) >> 2;
    let b: u8 = pixel & BLUE_MASK;
    (r, g, b)
}

fn rgb_to_pixel_clamped(r: u8, g: u8, b: u8) -> Pixel {
    r.min(RED_MAX) << RED_BIT_OFFSET | g.min(GREEN_MAX) << GREEN_BIT_OFFSET | b.min(BLUE_MAX)
}

pub(crate) fn add(prev_pixel: u8, pixel: u8) -> Pixel {
    let (pr, pg, pb) = pixel_to_rgb(prev_pixel);
    let (r, g, b) = pixel_to_rgb(pixel);
    rgb_to_pixel_clamped(pr + r, pg + g, pb + b)
}

pub(crate) fn subtract(prev_pixel: u8, pixel: u8) -> Pixel {
    let (pr, pg, pb) = pixel_to_rgb(prev_pixel);
    let (r, g, b) = pixel_to_rgb(pixel);
    rgb_to_pixel_clamped(pr - r, pg - g, pb - b)
}

pub(crate) fn multiply(prev_pixel: u8, pixel: u8) -> Pixel {
    let (pr, pg, pb) = pixel_to_rgb(prev_pixel);
    let (r, g, b) = pixel_to_rgb(pixel);
    rgb_to_pixel_clamped(pr * r, pg * g, pb * b)
}

pub(crate) fn divide(prev_pixel: u8, pixel: u8) -> Pixel {
    let (pr, pg, pb) = pixel_to_rgb(prev_pixel);
    let (r, g, b) = pixel_to_rgb(pixel);
    rgb_to_pixel_clamped(pr / r, pg / g, pb / b)
}

pub(crate) fn average(prev_pixel: u8, pixel: u8) -> Pixel {
    let (pr, pg, pb) = pixel_to_rgb(prev_pixel);
    let (r, g, b) = pixel_to_rgb(pixel);
    rgb_to_pixel_clamped((pr + r) / 2, (pg + g) / 2, (pb + b) / 2)
}

pub(crate) fn mask(prev_pixel: u8, pixel: u8, read_mask: u8, write_mask: u8) -> Pixel {
    (pixel & !write_mask) | (prev_pixel & write_mask & read_mask)
}
