use std::io::{Write, stdout};
use std::time::Duration;

use tokio::time::sleep;
use wasmtime::Linker;

use crate::kernel::ProcessContext;
use crate::process::Process;

mod event;
mod filesystem;
mod process;

// Convenience wrapper to help with casting wasm types.
pub fn get_memory<T>(ctx: &ProcessContext<T>, mem_ptr: i32, mem_len: u32) -> &'static mut [u8] {
    ctx.data().get_memory(mem_ptr as usize, mem_len as usize)
}

#[allow(mismatched_lifetime_syntaxes)]
fn get_str<T>(ctx: &ProcessContext<T>, str_ptr: i32, str_len: u32) -> Result<&'static str, i32> {
    let string = ctx.data().get_memory(str_ptr as usize, str_len as usize);
    let Ok(string) = str::from_utf8(string) else {
        return Err(-2);
    };
    Ok(string)
}

/// Loads all core system functions into the program.
pub fn load_system_functions<T>(linker: &mut Linker<Process<T>>) -> wasmtime::Result<()> {
    event::load_system_functions(linker)?;
    filesystem::load_system_functions(linker)?;
    process::load_system_functions(linker)?;

    // Kernel methods

    linker.func_wrap(
        "env",
        "debug_print",
        |ctx: ProcessContext<T>, str_ptr: i32, str_len: u32| {
            let _ = stdout().write_all(get_memory(&ctx, str_ptr, str_len));
        },
    )?;

    // Time

    linker.func_wrap_async(
        "env",
        "sleep",
        |_: ProcessContext<T>, (seconds,): (f64,)| {
            Box::new(async move {
                sleep(Duration::from_secs_f64(seconds)).await;
            })
        },
    )?;

    // Data

    linker.func_wrap("env", "get_data_length", |ctx: ProcessContext<T>| -> u32 {
        ctx.data().byte_data.as_ref().map(|d| d.len()).unwrap_or(0) as u32
    })?;

    linker.func_wrap(
        "env",
        "read_data",
        |ctx: ProcessContext<T>, buf_ptr: i32, buf_len: u32| {
            if let Some(data) = ctx.data().byte_data.as_ref() {
                let mut buf = get_memory(&ctx, buf_ptr, buf_len);
                let _ = buf.write_all(data);
            }
        },
    )?;

    // Filesystem
    Ok(())
}
