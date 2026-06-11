#![no_std]
#![no_main]

static mut FRAMEBUFFER: [u8; 384 * 216] = [0; 384 * 216];

extern "C" {
    #[link_name = "set_active_framebuffer"]
    fn set_active_framebuffer(framebuffer: *const u8);

    #[link_name = "get_mouse_x"]
    fn get_mouse_x() -> i32;

    #[link_name = "get_mouse_y"]
    fn get_mouse_y() -> i32;
}

#[no_mangle]
fn init() {
    unsafe {
        set_active_framebuffer(&raw const FRAMEBUFFER as *const u8);
    }
}

fn set_pixel_idx(idx: usize, color: u8) {
    unsafe {
        FRAMEBUFFER[idx] = color;
    }
}

fn set_pixel(x: usize, y: usize, color: u8) {
    if x >= 384 || y >= 216 {
        return;
    }

    let idx = y * 384 + x;
    set_pixel_idx(idx, color);
}

#[no_mangle]
fn update() {
    unsafe {
        let mx = get_mouse_x() as usize;
        let my = get_mouse_y() as usize;

        set_pixel(mx, my, 0b11111111);
        set_pixel(mx + 1, my, 0b01010101);
        set_pixel(mx, my + 1, 0b00110011);
        set_pixel(mx + 1, my + 1, 0b10101010);
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
