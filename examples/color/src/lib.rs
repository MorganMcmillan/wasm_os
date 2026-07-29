wit_bindgen::generate!({
    path: "main.wit"
});

export!(App);

struct App;

static mut COLOR: u8 = 0;

impl Guest for App {
    fn run() -> i32 {
        unsafe {
            let parent_pid = wasm_os::get_parent_pid();
            loop {
                wasm_os::send_event("set-color", &[COLOR], parent_pid);
                COLOR = COLOR.wrapping_add(1);
                wasm_os::sleep(0.1);
            }
        }
    }
}
