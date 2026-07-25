static mut FRAMEBUFFER: [u8; 384 * 216] = [0; 384 * 216];
static mut COLOR: u8 = 255;

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    #[link_name = "set_active_framebuffer"]
    fn set_active_framebuffer(framebuffer: *const u8);

    #[link_name = "get_mouse_x"]
    fn get_mouse_x() -> i32;

    #[link_name = "get_mouse_y"]
    fn get_mouse_y() -> i32;

    #[link_name = "yield_now"]
    fn yield_now();

    #[link_name = "add_event_handler"]
    fn add_event_handler(name_ptr: *const u8, name_len: i32) -> i32;

    #[link_name = "get_event_data"]
    fn get_event_data(buf_ptr: *mut u8, buf_len: i32);

    #[link_name = "spawn"]
    fn extern_spawn(path_ptr: *const u8, path_len: i32) -> i32;
}

fn spawn(path: &str) -> i32 {
    unsafe { extern_spawn(path.as_ptr(), path.len() as i32) }
}

#[unsafe(no_mangle)]
extern "C" fn set_color(_: i32) {
    let mut color_data: [u8; 1] = [0];
    unsafe {
        get_event_data(color_data.as_mut().as_mut_ptr(), 1);
        COLOR = color_data[0];
    }
}

// const DBG_MESSAGE: &str = "Drawing within Wasm!";

#[unsafe(no_mangle)]
fn run() -> i32 {
    unsafe {
        spawn("child.wasm");

        set_active_framebuffer(&raw const FRAMEBUFFER as *const u8);
        let name = "set_color";
        add_event_handler(name.as_ptr(), name.len() as i32);

        loop {
            let mx = get_mouse_x() as usize;
            let my = get_mouse_y() as usize;

            set_pixel(mx, my, COLOR);
            set_pixel(mx + 1, my, COLOR);
            set_pixel(mx, my + 1, COLOR);
            set_pixel(mx + 1, my + 1, COLOR);

            yield_now();
        }
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
