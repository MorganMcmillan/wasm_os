#[link(wasm_import_module = "driver_input")]
unsafe extern "C" {
    #[link_name = "get_mouse_x"]
    fn extern_get_mouse_x() -> i32;
    #[link_name = "get_mouse_y"]
    fn extern_get_mouse_y() -> i32;
}

pub fn get_mouse_x() -> u32 {
    unsafe { extern_get_mouse_x() as u32 }
}

pub fn get_mouse_y() -> u32 {
    unsafe { extern_get_mouse_y() as u32 }
}
