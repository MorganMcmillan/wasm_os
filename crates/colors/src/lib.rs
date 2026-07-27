fn draw_all_colors(framebuffer: &mut [u8; draw::SCREEN_SIZE]) {
    for (i, pixel) in framebuffer.iter_mut().enumerate() {
        *pixel = i as u8;
    }
}

#[unsafe(no_mangle)]
fn run() -> i32 {
    let mut framebuffer = [0u8; draw::SCREEN_SIZE];
    loop {
        draw_all_colors(&mut framebuffer);
        draw::upload_framebuffer(&framebuffer);
        wasm_os::yield_now();
    }
}
