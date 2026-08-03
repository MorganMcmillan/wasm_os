pub const FILE_WRITE: u8 = 0b1;
pub const FILE_APPEND: u8 = 0b10;
pub const FILE_CREATE: u8 = 0b100;
pub const FILE_TRUNCATE: u8 = 0b1000;

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    #[link_name = "debug_print"]
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

    #[link_name = "resend_event"]
    fn extern_resend_event(to_pid: i32) -> i32;

    #[link_name = "get_event_data"]
    fn extern_get_event_data(buf_ptr: *mut u8, buf_len: i32);

    #[link_name = "get_event_sender"]
    fn extern_get_event_sender() -> i32;

    #[link_name = "add_event_handler"]
    fn extern_add_event_handler(name_ptr: *const u8, name_len: i32) -> i32;

    #[link_name = "remove_event_handler"]
    fn extern_remove_event_handler(name_ptr: *const u8, name_len: i32) -> i32;

    #[link_name = "copy_process_memory"]
    fn extern_copy_process_memory(src_pid: i32, src: i32, dest: *mut u8, len: i32) -> i32;

    #[link_name = "set_process_name"]
    fn extern_set_process_name(name_ptr: *const u8, name_len: i32) -> i32;

    #[link_name = "get_process_label"]
    fn extern_get_process_label(buf_ptr: *mut u8, buf_len: i32) -> i32;

    #[link_name = "get_pid_by_name"]
    fn extern_get_pid_by_name(name_ptr: *const u8, name_len: i32) -> i32;

    #[link_name = "yield_now"]
    fn extern_yield_now();

    #[link_name = "is_directory"]
    fn extern_is_directory(path_ptr: *const u8, path_len: i32) -> i32;

    #[link_name = "is_file"]
    fn extern_is_file(path_ptr: *const u8, path_len: i32) -> i32;

    #[link_name = "file_exists"]
    fn extern_file_exists(path_ptr: *const u8, path_len: i32) -> i32;

    #[link_name = "file_size"]
    fn extern_file_size(path_ptr: *const u8, path_len: i32) -> i32;

    #[link_name = "file_created"]
    fn extern_file_created(path_ptr: *const u8, path_len: i32) -> i64;

    #[link_name = "file_accessed"]
    fn extern_file_accessed(path_ptr: *const u8, path_len: i32) -> i64;

    #[link_name = "file_modified"]
    fn extern_file_modified(path_ptr: *const u8, path_len: i32) -> i64;

    #[link_name = "open_file"]
    fn extern_open_file(path_ptr: *const u8, path_len: i32) -> i32;

    #[link_name = "read_file"]
    fn extern_read_file(fd: i32, buf_ptr: *mut u8, buf_len: i32) -> i32;

    #[link_name = "write_file"]
    fn extern_write_file(fd: i32, src_ptr: *mut u8, src_len: i32) -> i32;

    #[link_name = "seek"]
    fn extern_seek(fd: i32, offset: i32, from: i32) -> i32;

    #[link_name = "close_file"]
    fn extern_close_file(fd: i32) -> i32;

    #[link_name = "change_directory"]
    fn extern_change_directory(path_ptr: *const u8, path_len: i32) -> i32;

    #[link_name = "move_file"]
    fn extern_move_file(from_ptr: *const u8, from_len: i32, to_ptr: *const u8, to_len: i32) -> i32;

    #[link_name = "copy_file"]
    fn extern_copy_file(from_ptr: *const u8, from_len: i32, to_ptr: *const u8, to_len: i32) -> i32;

    #[link_name = "create_directory"]
    fn extern_create_directory(path_ptr: *const u8, path_len: i32) -> i32;
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

pub fn resend_event(to_pid: i32) -> i32 {
    unsafe { extern_resend_event(to_pid) }
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

pub fn copy_process_memory(src_pid: i32, src: i32, dest: &mut [u8]) -> i32 {
    unsafe { extern_copy_process_memory(src_pid, src, dest.as_mut_ptr(), dest.len() as i32) }
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

pub mod fs {
    use super::*;

    pub fn is_directory(path: &str) -> bool {
        unsafe { extern_is_directory(path.as_ptr(), path.len() as i32) != 0 }
    }

    pub fn is_file(path: &str) -> bool {
        unsafe { extern_is_file(path.as_ptr(), path.len() as i32) != 0 }
    }

    pub fn exists(path: &str) -> bool {
        unsafe { extern_file_exists(path.as_ptr(), path.len() as i32) != 0 }
    }

    pub fn size(path: &str) -> i32 {
        unsafe { extern_file_size(path.as_ptr(), path.len() as i32) }
    }

    pub fn created(path: &str) -> i64 {
        unsafe { extern_file_created(path.as_ptr(), path.len() as i32) }
    }

    pub fn accessed(path: &str) -> i64 {
        unsafe { extern_file_accessed(path.as_ptr(), path.len() as i32) }
    }
    pub fn modified(path: &str) -> i64 {
        unsafe { extern_file_modified(path.as_ptr(), path.len() as i32) }
    }

    pub fn open(path: &str) -> i32 {
        unsafe { extern_open_file(path.as_ptr(), path.len() as i32) }
    }

    pub fn read(fd: i32, buf: &mut [u8]) -> i32 {
        unsafe { extern_read_file(fd, buf.as_mut_ptr(), buf.len() as i32) }
    }

    pub fn write(fd: i32, buf: &mut [u8]) -> i32 {
        unsafe { extern_write_file(fd, buf.as_mut_ptr(), buf.len() as i32) }
    }

    pub fn seek(fd: i32, offset: i32, from: u8) -> i32 {
        unsafe { extern_seek(fd, offset, from as i32) }
    }

    pub fn close(fd: i32) -> i32 {
        unsafe { extern_close_file(fd) }
    }

    pub fn change_directory(path: &str) -> i32 {
        unsafe { extern_change_directory(path.as_ptr(), path.len() as i32) }
    }

    pub fn move_file(from: &str, to: &str) -> i32 {
        unsafe {
            extern_move_file(
                from.as_ptr(),
                from.len() as i32,
                to.as_ptr(),
                to.len() as i32,
            )
        }
    }

    pub fn copy_file(from: &str, to: &str) -> i32 {
        unsafe {
            extern_copy_file(
                from.as_ptr(),
                from.len() as i32,
                to.as_ptr(),
                to.len() as i32,
            )
        }
    }

    pub fn create_directory(path: &str) -> i32 {
        unsafe { extern_create_directory(path.as_ptr(), path.len() as i32) }
    }
}
