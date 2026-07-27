wit_bindgen::generate!({
    path: "main.wit"
});

export!(App);

struct App;

type Framebuffer = [u8; draw::SCREEN_SIZE];
static mut COLOR: u8 = 255;

fn set_pixel_idx(framebuffer: &mut Framebuffer, idx: usize, color: u8) {
    framebuffer[idx] = color;
}

fn set_pixel(framebuffer: &mut Framebuffer, x: usize, y: usize, color: u8) {
    if x >= 384 || y >= 216 {
        return;
    }

    let idx = y * 384 + x;
    set_pixel_idx(framebuffer, idx, color);
}
impl Guest for App {
    fn run() -> i32 {
        let mut frambuffer = [0; draw::SCREEN_SIZE];

        loop {
            let mx = input::get_mouse_x() as usize;
            let my = input::get_mouse_y() as usize;

            unsafe {
                set_pixel(&mut frambuffer, mx, my, COLOR);
                set_pixel(&mut frambuffer, mx + 1, my, COLOR);
                set_pixel(&mut frambuffer, mx, my + 1, COLOR);
                set_pixel(&mut frambuffer, mx + 1, my + 1, COLOR);
            }

            draw::upload_framebuffer(&frambuffer);
            wasm_os::yield_now();
        }
    }

    fn set_color(length: u32) -> () {
        let color_data = wasm_os::get_event_data(length);
    }
}
