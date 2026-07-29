pub const FILE_WRITE: u8 = 0b1;
pub const FILE_APPEND: u8 = 0b10;
pub const FILE_CREATE: u8 = 0b100;
pub const FILE_TRUNCATE: u8 = 0b1000;

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn extern_debug_print(str_ptr: *const u8, str_len: i32) -> i32;

    #[link_name = "get_pid"]
    fn extern_get_pid() -> i32;

    #[link_name = "get_parent_pid"]
    fn extern_get_parent_pid() -> i32;

    #[link_name = "spawn"]
    fn extern_spawn(path_ptr: *const u8, path_len: i32) -> i32;

    #[link_name = "exit"]
    fn extern_exit(code: i32);

    #[link_name = "sleep"]
    fn extern_sleep(second: f64);

    #[link_name = "send_event"]
    fn extern_send_event(
        name_ptr: *const u8,
        name_len: i32,
        data_ptr: *const u8,
        data_len: i32,
        to_pid: i32,
    ) -> i32;

    #[link_name = "get_event_data"]
    fn extern_get_event_data(buf_ptr: *mut u8, buf_len: i32);

    #[link_name = "get_event_sender"]
    fn extern_get_event_sender() -> i32;

    #[link_name = "add_event_handler"]
    fn extern_add_event_handler(name_ptr: *const u8, name_len: i32) -> i32;

    #[link_name = "remove_event_handler"]
    fn extern_remove_event_handler(name_ptr: *const u8, name_len: i32) -> i32;

    #[link_name = "proc_memcpy"]
    fn extern_proc_memcpy(src_pid: i32, src: i32, dest: *mut u8, len: i32) -> i32;

    #[link_name = "set_process_name"]
    fn extern_set_process_name(name_ptr: *const u8, name_len: i32) -> i32;

    #[link_name = "get_process_label"]
    fn extern_get_process_label(buf_ptr: *mut u8, buf_len: i32) -> i32;

    #[link_name = "get_pid_by_name"]
    fn extern_get_pid_by_name(name_ptr: *const u8, name_len: i32) -> i32;

    #[link_name = "yield_now"]
    fn extern_yield_now();
}

pub fn debug_print(bytes: &[u8]) -> i32 {
    unsafe { extern_debug_print(bytes.as_ptr(), bytes.len() as i32) }
}

pub fn get_pid() -> i32 {
    unsafe { extern_get_pid() }
}

pub fn get_parent_pid() -> i32 {
    unsafe { extern_get_parent_pid() }
}

pub fn spawn(path: &str) -> i32 {
    unsafe { extern_spawn(path.as_ptr(), path.len() as i32) }
}

pub fn exit(code: i32) -> ! {
    unsafe {
        extern_exit(code);
    }
    panic!("Somehow, the process did not exit")
}

pub fn sleep(seconds: f64) {
    unsafe { extern_sleep(seconds) }
}

pub fn send_event(name: &str, data: &[u8], to_pid: i32) -> i32 {
    unsafe {
        extern_send_event(
            name.as_ptr(),
            name.len() as i32,
            data.as_ptr(),
            data.len() as i32,
            to_pid,
        )
    }
}

pub fn get_event_data(buffer: &mut [u8]) {
    unsafe { extern_get_event_data(buffer.as_mut_ptr(), buffer.len() as i32) }
}

pub fn get_event_sender() -> i32 {
    unsafe { extern_get_event_sender() }
}

pub fn add_event_handler(name: &str) -> i32 {
    unsafe { extern_add_event_handler(name.as_ptr(), name.len() as i32) }
}

pub fn remove_event_handler(name: &str) -> i32 {
    unsafe { extern_remove_event_handler(name.as_ptr(), name.len() as i32) }
}

pub fn proc_memcpy(src_pid: i32, src: i32, dest: &mut [u8]) -> i32 {
    unsafe { extern_proc_memcpy(src_pid, src, dest.as_mut_ptr(), dest.len() as i32) }
}

pub fn set_process_name(name: &str) -> i32 {
    unsafe { extern_set_process_name(name.as_ptr(), name.len() as i32) }
}

pub fn get_process_label(buffer: &mut [u8]) -> i32 {
    unsafe { extern_get_process_label(buffer.as_mut_ptr(), buffer.len() as i32) }
}

pub fn get_pid_by_name(name: &str) -> i32 {
    unsafe { extern_get_pid_by_name(name.as_ptr(), name.len() as i32) }
}

pub fn yield_now() {
    unsafe { extern_yield_now() }
}
