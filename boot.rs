#![no_std]
#![no_main]

static mut FRAMEBUFFER: [u8; 384 * 216] = [0; 384 * 216];

#[link(wasm_import_module = "env")]
extern "C" {
    #[link_name = "set_active_framebuffer"]
    fn set_active_framebuffer(framebuffer: *const u8);

    #[link_name = "get_mouse_x"]
    fn get_mouse_x() -> i32;

    #[link_name = "get_mouse_y"]
    fn get_mouse_y() -> i32;

    #[link_name = "yield_now"]
    fn yield_now();
}

#[no_mangle]
fn run() -> i32 {
    unsafe {
        set_active_framebuffer(&raw const FRAMEBUFFER as *const u8);

        let mx = get_mouse_x() as usize;
        let my = get_mouse_y() as usize;

        set_pixel(mx, my, WHITE);
        set_pixel(mx + 1, my, WHITE);
        set_pixel(mx, my + 1, WHITE);
        set_pixel(mx + 1, my + 1, WHITE);

        yield_now();
    }
    0
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

const WHITE: u8 = 255;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
