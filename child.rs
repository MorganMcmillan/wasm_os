#![no_std]
#![no_main]

#[link(wasm_import_module = "env")]
extern "C" {
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

    #[link_name = "send_event"]
    fn get_parent_pid() -> i32;
}

fn send_event_safe(name: &str, data: &[u8], pid: i32) {
    send_event(name.as_ptr, name.len(), data.as_ptr(), data.len(), pid);
}

#[no_mangle]
fn run() -> i32 {
    let parent_pid = get_parent_pid();
    loop {
        send_event("set_color", &[255], parent_pid);
        sleep(1.0);
        send_event("set_color", &[80], parent_pid);
        sleep(1.0);
        send_event("set_color", &[127], parent_pid);
        sleep(1.0);
    }
}
