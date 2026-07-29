fn draw_all_colors(framebuffer: &mut [u8; screen::SCREEN_SIZE], offset: u8) {
    for (i, pixel) in framebuffer.iter_mut().enumerate() {
        *pixel = (i as u8).wrapping_add(offset);
    }
}

#[unsafe(no_mangle)]
fn run() -> i32 {
    let mut framebuffer = [0u8; screen::SCREEN_SIZE];
    let mut offset = 0u8;
    loop {
        draw_all_colors(&mut framebuffer, offset);
        offset = offset.wrapping_add(1);
        screen::upload_framebuffer(&framebuffer);
        // wasm_os::yield_now();
    }
}
