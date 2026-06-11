#![no_std]
#![no_main]

static mut FRAMEBUFFER: [u8; 384 * 216] = [0; 384 * 216];

extern "C" {
    #[link_name = "set_active_framebuffer"]
    fn set_active_framebuffer(framebuffer: *const u8);
}

static mut j: usize = 0;

#[no_mangle]
fn init() {
    unsafe {
        set_active_framebuffer(&raw const FRAMEBUFFER as *const u8);
    }
}

fn set_pixel(idx: usize, color: u8) {
    unsafe {
        FRAMEBUFFER[idx] = color;
    }
}

#[no_mangle]
fn update() {
    unsafe {
        for i in 0..(384 * 216) {
            set_pixel(i, (i + j) as u8);
        }

        j += 3;
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
