#[link(wasm_import_module = "env")]
unsafe extern "C" {
    #[link_name = "sleep"]
    fn sleep(seconds: f64);

    #[link_name = "send_event"]
    fn send_event(
        name_ptr: *const u8,
        name_len: i32,
        data_ptr: *const u8,
        data_len: i32,
        pid: i32,
    ) -> i32;

    #[link_name = "get_parent_pid"]
    fn get_parent_pid() -> i32;
}

fn send_event_safe(name: &str, data: &[u8], pid: i32) {
    unsafe {
        send_event(
            name.as_ptr(),
            name.len() as i32,
            data.as_ptr(),
            data.len() as i32,
            pid,
        );
    }
}

static mut COLOR: u8 = 0;

#[unsafe(no_mangle)]
fn run() -> i32 {
    unsafe {
        let parent_pid = get_parent_pid();
        loop {
            send_event_safe("set_color", &[COLOR], parent_pid);
            COLOR = COLOR.wrapping_add(1);
            sleep(0.1);
        }
    }
}
