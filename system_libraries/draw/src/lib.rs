pub const SCREEN_WIDTH: usize = 384;
pub const SCREEN_HEIGHT: usize = 216;
pub const SCREEN_SIZE: usize = SCREEN_WIDTH * SCREEN_HEIGHT;

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    #[link_name = "upload_framebuffer"]
    fn extern_upload_framebuffer(framebuffer: *const u8);
}

pub fn upload_framebuffer(framebuffer: &[u8]) {
    unsafe {
        extern_upload_framebuffer(framebuffer.as_ptr());
    }
}
