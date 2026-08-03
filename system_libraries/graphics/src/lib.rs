#[link(wasm_import_module = "env")]
unsafe extern "C" {
    #[link_name = "set_draw_region"]
    fn extern_set_draw_region(buffer: *mut u8, width: u32, height: u32);
    #[link_name = "clear_draw_region"]
    fn extern_clear_draw_region(color: u8);
    #[link_name = "set_transparency_color"]
    fn extern_set_transparency_color(color: u8) -> u8;
    #[link_name = "set_camera"]
    fn extern_set_camera(x: i32, y: i32);
    #[link_name = "get_camera_x"]
    fn extern_get_camera_x() -> i32;
    #[link_name = "get_camera_y"]
    fn extern_get_camera_y() -> i32;
    #[link_name = "set_font"]
    fn extern_set_font(font: *const u8);
    #[link_name = "use_default_font"]
    fn extern_use_default_font();
    #[link_name = "draw_pixel"]
    fn extern_draw_pixel(x: i32, y: i32, color: u8);
    #[link_name = "draw_hline"]
    fn extern_draw_hline(x: i32, y: i32, width: u32, color: u8);
    #[link_name = "draw_vline"]
    fn extern_draw_vline(x: i32, y: i32, height: u32, color: u8);
    #[link_name = "draw_line"]
    fn extern_draw_line(x1: i32, y1: i32, x2: i32, y2: i32, color: u8);
    #[link_name = "draw_textured_line"]
    fn extern_draw_textured_line(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        texture: *const u8,
        tex_width: u32,
        tex_height: u32,
        tex_x: f32,
        tex_y: f32,
        tex_dx: f32,
        tex_dy: f32,
    );
    #[link_name = "draw_rectangle"]
    fn extern_draw_rectangle(x: i32, y: i32, width: u32, height: u32, color: u8);
    #[link_name = "draw_filled_rectangle"]
    fn extern_draw_filled_rectangle(x: i32, y: i32, width: u32, height: u32, color: u8);
    #[link_name = "draw_round_rectangle"]
    fn extern_draw_round_rectangle(x: i32, y: i32, width: u32, height: u32, radius: u32, color: u8);
    #[link_name = "draw_filled_round_rectangle"]
    fn extern_draw_filled_round_rectangle(
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        radius: u32,
        color: u8,
    );
    #[link_name = "draw_circle"]
    fn extern_draw_circle(x: i32, y: i32, radius: u32, color: u8);
    #[link_name = "draw_filled_circle"]
    fn extern_draw_filled_circle(x: i32, y: i32, radius: u32, color: u8);
    #[link_name = "draw_ellipse"]
    fn extern_draw_ellipse(x: i32, y: i32, x_radius: u32, y_radius: u32, color: u8);
    #[link_name = "draw_filled_ellipse"]
    fn extern_draw_filled_ellipse(x: i32, y: i32, x_radius: u32, y_radius: u32, color: u8);
    #[link_name = "draw_sprite"]
    fn extern_draw_sprite(x: i32, y: i32, sprite: *const u8, spr_width: u32, spr_height: u32);
    #[link_name = "draw_text"]
    fn extern_draw_text(text_ptr: *const u8, text_len: u32, color: u8);
}

pub fn set_draw_region(buffer: *mut u8, width: u32, height: u32) {
    unsafe {
        extern_set_draw_region(buffer, width, height);
    }
}

pub fn clear_draw_region(color: u8) {
    unsafe {
        extern_clear_draw_region(color);
    }
}

pub fn set_transparency_color(color: u8) -> u8 {
    unsafe { extern_set_transparency_color(color) }
}

pub fn set_camera(x: i32, y: i32) {
    unsafe {
        extern_set_camera(x, y);
    }
}

pub fn get_camera_x() -> i32 {
    unsafe { extern_get_camera_x() }
}

pub fn get_camera_y() -> i32 {
    unsafe { extern_get_camera_y() }
}

pub fn set_font(font: &[u8; 8 * 256]) {
    unsafe {
        extern_set_font(font.as_ptr());
    }
}

pub fn use_default_font() {
    unsafe {
        extern_use_default_font();
    }
}

pub fn draw_pixel(x: i32, y: i32, color: u8) {
    unsafe {
        extern_draw_pixel(x, y, color);
    }
}

pub fn draw_hline(x: i32, y: i32, width: u32, color: u8) {
    unsafe {
        extern_draw_hline(x, y, width, color);
    }
}

pub fn draw_vline(x: i32, y: i32, height: u32, color: u8) {
    unsafe {
        extern_draw_vline(x, y, height, color);
    }
}

pub fn draw_line(x1: i32, y1: i32, x2: i32, y2: i32, color: u8) {
    unsafe {
        extern_draw_line(x1, y1, x2, y2, color);
    }
}

pub fn draw_textured_line(
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    texture: *const u8,
    tex_width: u32,
    tex_height: u32,
    tex_x: f32,
    tex_y: f32,
    tex_dx: f32,
    tex_dy: f32,
) {
    unsafe {
        extern_draw_textured_line(
            x1, y1, x2, y2, texture, tex_width, tex_height, tex_x, tex_y, tex_dx, tex_dy,
        );
    }
}

pub fn draw_rectangle(x: i32, y: i32, width: u32, height: u32, color: u8) {
    unsafe {
        extern_draw_rectangle(x, y, width, height, color);
    }
}

pub fn draw_filled_rectangle(x: i32, y: i32, width: u32, height: u32, color: u8) {
    unsafe {
        extern_draw_filled_rectangle(x, y, width, height, color);
    }
}

pub fn draw_round_rectangle(x: i32, y: i32, width: u32, height: u32, radius: u32, color: u8) {
    unsafe {
        extern_draw_round_rectangle(x, y, width, height, radius, color);
    }
}

pub fn draw_filled_round_rectangle(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    radius: u32,
    color: u8,
) {
    unsafe {
        extern_draw_filled_round_rectangle(x, y, width, height, radius, color);
    }
}

pub fn draw_circle(x: i32, y: i32, radius: u32, color: u8) {
    unsafe {
        extern_draw_circle(x, y, radius, color);
    }
}

pub fn draw_filled_circle(x: i32, y: i32, radius: u32, color: u8) {
    unsafe {
        extern_draw_filled_circle(x, y, radius, color);
    }
}

pub fn draw_ellipse(x: i32, y: i32, x_radius: u32, y_radius: u32, color: u8) {
    unsafe {
        extern_draw_ellipse(x, y, x_radius, y_radius, color);
    }
}

pub fn draw_filled_ellipse(x: i32, y: i32, x_radius: u32, y_radius: u32, color: u8) {
    unsafe {
        extern_draw_filled_ellipse(x, y, x_radius, y_radius, color);
    }
}

pub fn draw_sprite(x: i32, y: i32, sprite: *const u8, spr_width: u32, spr_height: u32) {
    unsafe {
        extern_draw_sprite(x, y, sprite, spr_width, spr_height);
    }
}

pub fn draw_text(text: &[u8], color: u8) {
    unsafe {
        extern_draw_text(text.as_ptr(), text.len() as u32, color);
    }
}
