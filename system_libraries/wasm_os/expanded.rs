#![feature(prelude_import)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2024::*;
#[allow(unused_unsafe, clippy::all)]
#[allow(async_fn_in_trait)]
pub fn debug_print(message: &str) -> () {
    unsafe {
        let vec0 = message;
        let ptr0 = vec0.as_ptr().cast::<u8>();
        let len0 = vec0.len();
        unsafe extern "C" fn wit_import1(_: *mut u8, _: usize) {
            ::core::panicking::panic("internal error: entered unreachable code")
        }
        wit_import1(ptr0.cast_mut(), len0);
    }
}
#[allow(unused_unsafe, clippy::all)]
#[allow(async_fn_in_trait)]
pub fn get_pid() -> i32 {
    unsafe {
        unsafe extern "C" fn wit_import0() -> i32 {
            ::core::panicking::panic("internal error: entered unreachable code")
        }
        let ret = wit_import0();
        ret
    }
}
#[allow(unused_unsafe, clippy::all)]
#[allow(async_fn_in_trait)]
pub fn get_parent_pid() -> i32 {
    unsafe {
        unsafe extern "C" fn wit_import0() -> i32 {
            ::core::panicking::panic("internal error: entered unreachable code")
        }
        let ret = wit_import0();
        ret
    }
}
#[allow(unused_unsafe, clippy::all)]
#[allow(async_fn_in_trait)]
pub fn spawn(path: &str) -> i32 {
    unsafe {
        let vec0 = path;
        let ptr0 = vec0.as_ptr().cast::<u8>();
        let len0 = vec0.len();
        unsafe extern "C" fn wit_import1(_: *mut u8, _: usize) -> i32 {
            ::core::panicking::panic("internal error: entered unreachable code")
        }
        let ret = wit_import1(ptr0.cast_mut(), len0);
        ret
    }
}
#[allow(unused_unsafe, clippy::all)]
#[allow(async_fn_in_trait)]
pub fn exit(code: i32) -> () {
    unsafe {
        unsafe extern "C" fn wit_import0(_: i32) {
            ::core::panicking::panic("internal error: entered unreachable code")
        }
        wit_import0(_rt::as_i32(&code));
    }
}
#[allow(unused_unsafe, clippy::all)]
#[allow(async_fn_in_trait)]
pub fn sleep(seconds: f64) -> () {
    unsafe {
        unsafe extern "C" fn wit_import0(_: f64) {
            ::core::panicking::panic("internal error: entered unreachable code")
        }
        wit_import0(_rt::as_f64(&seconds));
    }
}
#[allow(unused_unsafe, clippy::all)]
#[allow(async_fn_in_trait)]
pub fn send_event(name: &str, data: &[u8], pid: i32) -> i32 {
    unsafe {
        let vec0 = name;
        let ptr0 = vec0.as_ptr().cast::<u8>();
        let len0 = vec0.len();
        let vec1 = data;
        let ptr1 = vec1.as_ptr().cast::<u8>();
        let len1 = vec1.len();
        unsafe extern "C" fn wit_import2(
            _: *mut u8,
            _: usize,
            _: *mut u8,
            _: usize,
            _: i32,
        ) -> i32 {
            ::core::panicking::panic("internal error: entered unreachable code")
        }
        let ret = wit_import2(
            ptr0.cast_mut(),
            len0,
            ptr1.cast_mut(),
            len1,
            _rt::as_i32(&pid),
        );
        ret
    }
}
#[allow(unused_unsafe, clippy::all)]
#[allow(async_fn_in_trait)]
pub fn get_event_data() -> _rt::Vec<u8> {
    unsafe {
        #[repr(align(8))]
        struct RetArea(
            [::core::mem::MaybeUninit<u8>; 2 * ::core::mem::size_of::<*const u8>()],
        );
        let mut ret_area = RetArea(
            [::core::mem::MaybeUninit::uninit(); 2 * ::core::mem::size_of::<*const u8>()],
        );
        let ptr0 = ret_area.0.as_mut_ptr().cast::<u8>();
        unsafe extern "C" fn wit_import1(_: *mut u8) {
            ::core::panicking::panic("internal error: entered unreachable code")
        }
        wit_import1(ptr0);
        let l2 = *ptr0.add(0).cast::<*mut u8>();
        let l3 = *ptr0.add(::core::mem::size_of::<*const u8>()).cast::<usize>();
        let len4 = l3;
        let result5 = <_ as From<
            _rt::Vec<_>,
        >>::from(_rt::Vec::from_raw_parts(l2.cast(), len4, len4));
        result5
    }
}
#[allow(unused_unsafe, clippy::all)]
#[allow(async_fn_in_trait)]
pub fn get_event_sender() -> i32 {
    unsafe {
        unsafe extern "C" fn wit_import0() -> i32 {
            ::core::panicking::panic("internal error: entered unreachable code")
        }
        let ret = wit_import0();
        ret
    }
}
#[allow(unused_unsafe, clippy::all)]
#[allow(async_fn_in_trait)]
pub fn add_event_handler(name: &str) -> i32 {
    unsafe {
        let vec0 = name;
        let ptr0 = vec0.as_ptr().cast::<u8>();
        let len0 = vec0.len();
        unsafe extern "C" fn wit_import1(_: *mut u8, _: usize) -> i32 {
            ::core::panicking::panic("internal error: entered unreachable code")
        }
        let ret = wit_import1(ptr0.cast_mut(), len0);
        ret
    }
}
#[allow(unused_unsafe, clippy::all)]
#[allow(async_fn_in_trait)]
pub fn remove_event_handler(name: &str) -> i32 {
    unsafe {
        let vec0 = name;
        let ptr0 = vec0.as_ptr().cast::<u8>();
        let len0 = vec0.len();
        unsafe extern "C" fn wit_import1(_: *mut u8, _: usize) -> i32 {
            ::core::panicking::panic("internal error: entered unreachable code")
        }
        let ret = wit_import1(ptr0.cast_mut(), len0);
        ret
    }
}
#[allow(unused_unsafe, clippy::all)]
#[allow(async_fn_in_trait)]
pub fn set_process_name(name: &str) -> i32 {
    unsafe {
        let vec0 = name;
        let ptr0 = vec0.as_ptr().cast::<u8>();
        let len0 = vec0.len();
        unsafe extern "C" fn wit_import1(_: *mut u8, _: usize) -> i32 {
            ::core::panicking::panic("internal error: entered unreachable code")
        }
        let ret = wit_import1(ptr0.cast_mut(), len0);
        ret
    }
}
#[allow(unused_unsafe, clippy::all)]
#[allow(async_fn_in_trait)]
pub fn get_process_label() -> _rt::String {
    unsafe {
        #[repr(align(8))]
        struct RetArea(
            [::core::mem::MaybeUninit<u8>; 2 * ::core::mem::size_of::<*const u8>()],
        );
        let mut ret_area = RetArea(
            [::core::mem::MaybeUninit::uninit(); 2 * ::core::mem::size_of::<*const u8>()],
        );
        let ptr0 = ret_area.0.as_mut_ptr().cast::<u8>();
        unsafe extern "C" fn wit_import1(_: *mut u8) {
            ::core::panicking::panic("internal error: entered unreachable code")
        }
        wit_import1(ptr0);
        let l2 = *ptr0.add(0).cast::<*mut u8>();
        let l3 = *ptr0.add(::core::mem::size_of::<*const u8>()).cast::<usize>();
        let len4 = l3;
        let bytes4 = _rt::Vec::from_raw_parts(l2.cast(), len4, len4);
        let result5 = _rt::string_lift(bytes4);
        result5
    }
}
#[allow(unused_unsafe, clippy::all)]
#[allow(async_fn_in_trait)]
pub fn get_pid_by_name(name: &str) -> i32 {
    unsafe {
        let vec0 = name;
        let ptr0 = vec0.as_ptr().cast::<u8>();
        let len0 = vec0.len();
        unsafe extern "C" fn wit_import1(_: *mut u8, _: usize) -> i32 {
            ::core::panicking::panic("internal error: entered unreachable code")
        }
        let ret = wit_import1(ptr0.cast_mut(), len0);
        ret
    }
}
#[allow(unused_unsafe, clippy::all)]
#[allow(async_fn_in_trait)]
pub fn yield_now() -> () {
    unsafe {
        unsafe extern "C" fn wit_import0() {
            ::core::panicking::panic("internal error: entered unreachable code")
        }
        wit_import0();
    }
}
#[doc(hidden)]
#[allow(non_snake_case, unused_unsafe)]
pub unsafe fn _export_run_cabi<T_: Guest>() -> i32 {
    unsafe {
        let result0 = { T_::run() };
        _rt::as_i32(result0)
    }
}
pub trait Guest {
    #[allow(async_fn_in_trait)]
    fn run() -> i32;
}
#[doc(hidden)]
pub(crate) use __export_world_system_cabi;
mod _rt {
    #![allow(dead_code, unused_imports, clippy::all)]
    pub fn as_i32<T: AsI32>(t: T) -> i32 {
        t.as_i32()
    }
    pub trait AsI32 {
        fn as_i32(self) -> i32;
    }
    impl<'a, T: Copy + AsI32> AsI32 for &'a T {
        fn as_i32(self) -> i32 {
            (*self).as_i32()
        }
    }
    impl AsI32 for i32 {
        #[inline]
        fn as_i32(self) -> i32 {
            self as i32
        }
    }
    impl AsI32 for u32 {
        #[inline]
        fn as_i32(self) -> i32 {
            self as i32
        }
    }
    impl AsI32 for i16 {
        #[inline]
        fn as_i32(self) -> i32 {
            self as i32
        }
    }
    impl AsI32 for u16 {
        #[inline]
        fn as_i32(self) -> i32 {
            self as i32
        }
    }
    impl AsI32 for i8 {
        #[inline]
        fn as_i32(self) -> i32 {
            self as i32
        }
    }
    impl AsI32 for u8 {
        #[inline]
        fn as_i32(self) -> i32 {
            self as i32
        }
    }
    impl AsI32 for char {
        #[inline]
        fn as_i32(self) -> i32 {
            self as i32
        }
    }
    impl AsI32 for usize {
        #[inline]
        fn as_i32(self) -> i32 {
            self as i32
        }
    }
    pub fn as_f64<T: AsF64>(t: T) -> f64 {
        t.as_f64()
    }
    pub trait AsF64 {
        fn as_f64(self) -> f64;
    }
    impl<'a, T: Copy + AsF64> AsF64 for &'a T {
        fn as_f64(self) -> f64 {
            (*self).as_f64()
        }
    }
    impl AsF64 for f64 {
        #[inline]
        fn as_f64(self) -> f64 {
            self as f64
        }
    }
    pub use alloc_crate::vec::Vec;
    pub use alloc_crate::string::String;
    pub unsafe fn string_lift(bytes: Vec<u8>) -> String {
        if true {
            String::from_utf8(bytes).unwrap()
        } else {
            unsafe { String::from_utf8_unchecked(bytes) }
        }
    }
    extern crate alloc as alloc_crate;
}
#[doc(inline)]
pub(crate) use __export_system_impl as export;
#[inline(never)]
#[doc(hidden)]
pub fn __link_custom_section_describing_imports() {
    wit_bindgen::rt::maybe_link_cabi_realloc();
}
const _: &[u8] = b"package wasm-os:system;\n\nworld system {\n    import debug-print: func(message: string);\n    import get-pid: func() -> s32;\n    import get-parent-pid: func() -> s32;\n    import spawn: func(path: string) -> s32;\n    import exit: func(code: s32);\n    import sleep: func(seconds: f64);\n    import send-event: func(name: string, data: list<u8>, pid: s32) -> s32;\n    import get-event-data: func() -> list<u8>;\n    import get-event-sender: func() -> s32;\n    import add-event-handler: func(name: string) -> s32;\n    import remove-event-handler: func(name: string) -> s32;\n    import set-process-name: func(name: string) -> s32;\n    import get-process-label: func() -> string;\n    import get-pid-by-name: func(name: string) -> s32;\n    import yield-now: func();\n    export run: func() -> s32;\n}\n";
