#[link(wasm_import_module = "env")]
extern "C" {
    #[link_name = "sleep"]
    fn sleep(seconds: f64);

    #[link_name = "send_event"]
    fn send_event(pid: i32, name_ptr: *const u8, name_len: i32, data_ptr: *const u8, data_len: i32, pid: i32) -> i32;
}

