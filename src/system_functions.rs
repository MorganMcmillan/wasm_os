use std::io::{Write, stdout};
use std::time::Duration;

use tokio::task::yield_now;
use tokio::time::sleep;
use wasmtime::Linker;

use crate::KERNEL;
use crate::async_file::AsyncFile;
use crate::kernel::{Pid, ProcessContext};
use crate::process::Process;

#[allow(mismatched_lifetime_syntaxes)]
pub fn get_memory(ctx: *const ProcessContext, mem_ptr: i32, mem_len: i32) -> Result<&[u8], i32> {
    unsafe {
        let process = KERNEL.get_process_mut((*ctx).data().pid).unwrap();

        let mem = process.get_memory(mem_ptr as usize, mem_len as usize);
        Ok(mem)
    }
}

#[allow(mismatched_lifetime_syntaxes)]
fn get_str(ctx: *const ProcessContext, str_ptr: i32, str_len: i32) -> Result<&str, i32> {
    let string = get_memory(ctx, str_ptr, str_len)?;
    let Ok(string) = str::from_utf8(string) else {
        return Err(-2);
    };
    Ok(string)
}

/// Loads all core system functions into the program.
/// TODO: allow drivers to register their own functions through this or a similar method.
pub fn load_system_functions(linker: &mut Linker<Process>) -> wasmtime::Result<()> {
    // Kernel methods

    linker.func_wrap(
        "env",
        "debug_print",
        |caller: ProcessContext, str_ptr: i32, str_len: i32| {
            if let Ok(bytes) = get_memory(&caller, str_ptr, str_len) {
                // Is okay if it fails (although an error code would be nice)
                let _ = stdout().write_all(bytes);
            }
        },
    )?;

    linker.func_wrap("env", "get_pid", |ctx: ProcessContext| -> i32 {
        ctx.data().pid.as_i32()
    })?;

    linker.func_wrap("env", "get_parent_pid", |ctx: ProcessContext| -> i32 {
        ctx.data().parent_pid.as_i32()
    })?;

    // Return Pid
    linker.func_wrap_async(
        "env",
        "spawn",
        |ctx: ProcessContext, (path_ptr, path_len): (i32, i32)| {
            let result = get_str(&ctx, path_ptr, path_len);
            let pid = ctx.data().pid;

            Box::new(async move {
                unsafe {
                    let path = match result {
                        Ok(p) => p,
                        Err(_) => return 0,
                    };

                    match KERNEL
                        .run_process(path, pid, AsyncFile::Null, AsyncFile::Null, AsyncFile::Null)
                        .await
                    {
                        Ok(id) => id.as_i32(),
                        Err(_) => 0,
                    }
                }
            })
        },
    )?;

    linker.func_wrap_async(
        "env",
        "exit",
        |mut caller: ProcessContext, (code,): (i32,)| {
            Box::new(async move {
                // Await join handle to end program execution.
                let _ = caller.data_mut().join_handle.as_mut().unwrap().await;
                caller.data_mut().exit_code = Some(code as u16);
            })
        },
    )?;

    // Time

    linker.func_wrap_async("env", "sleep", |_: ProcessContext, (seconds,): (f64,)| {
        Box::new(async move {
            sleep(Duration::from_secs_f64(seconds)).await;
        })
    })?;

    // Inter-process communication

    linker.func_wrap(
        "env",
        "send_event",
        |caller: ProcessContext,
         name_ptr: i32,
         name_len: i32,
         data_ptr: i32,
         data_len: i32,
         to_pid: i32|
         -> i32 {
            let name = match get_str(&caller, name_ptr, name_len) {
                Ok(name) => name,
                Err(e) => return e,
            };

            let data = match get_memory(&caller, data_ptr, data_len) {
                Ok(d) => d,
                Err(e) => return e,
            };

            unsafe { KERNEL.send_event(name, data, caller.data().pid, Pid::from_i32(to_pid)) }
        },
    )?;

    linker.func_wrap(
        "env",
        "resend_event",
        |ctx: ProcessContext, to_pid: i32| -> i32 {
            unsafe {
                let event = KERNEL.get_current_event();
                let pid = ctx.data().pid;
                KERNEL.resend_event(event, pid, Pid::from_i32(to_pid))
            }
        },
    )?;

    linker.func_wrap(
        "env",
        "get_event_data",
        |caller: ProcessContext, buf_ptr: i32, buf_len: i32| unsafe {
            let buf_ptr = buf_ptr as usize;
            let buf_len = buf_len as usize;

            let event = KERNEL.get_current_event();
            let data = event.data();
            let process = KERNEL.get_process_mut(caller.data().pid).unwrap();
            if data.len() < buf_len {
                process.set_memory(buf_ptr, data);
            } else {
                process.set_memory(buf_ptr, &data[..buf_len]);
            }
        },
    )?;

    linker.func_wrap("env", "get_event_sender", |_: ProcessContext| -> i32 {
        let event = unsafe { KERNEL.get_current_event() };
        event.sent_by_pid.as_i32()
    })?;

    linker.func_wrap(
        "env",
        "add_event_handler",
        |mut caller: ProcessContext, name_ptr: i32, name_len: i32| -> i32 {
            let name = match get_str(&caller, name_ptr, name_len) {
                Ok(n) => n,
                Err(e) => return e,
            };
            let interned_name = unsafe { KERNEL.intern_event_name(name) };

            let handler = caller
                .get_export(name)
                .unwrap()
                .into_func()
                .unwrap()
                .typed::<i32, ()>(&caller)
                .unwrap();

            caller.data_mut().add_event_handler(interned_name, handler);
            0
        },
    )?;

    linker.func_wrap(
        "env",
        "remove_event_handler",
        |mut caller: ProcessContext, name_ptr: i32, name_len: i32| -> i32 {
            let name = match get_str(&caller, name_ptr, name_len) {
                Ok(o) => o,
                Err(e) => return e,
            };
            let interned_name = unsafe { KERNEL.intern_event_name(name) };

            caller.data_mut().remove_event_handler(interned_name);
            0
        },
    )?;

    linker.func_wrap(
        "env",
        "proc_memcpy",
        |caller: ProcessContext, src_pid: i32, src: i32, dest: i32, len: i32| -> i32 {
            let src_pid = Pid::from_i32(src_pid);
            let src = src as usize;
            let dest = dest as usize;
            let len = len as usize;

            unsafe {
                let Some(src_proc) = KERNEL.get_process(src_pid) else {
                    return 1;
                };

                KERNEL
                    .get_process_mut(caller.data().pid)
                    .unwrap()
                    .set_memory(dest, src_proc.get_memory(src, len));
            }
            0
        },
    )?;

    linker.func_wrap(
        "env",
        "set_process_name",
        |caller: ProcessContext, name_ptr: i32, name_len: i32| -> i32 {
            let name = match get_str(&caller, name_ptr, name_len) {
                Ok(n) => n,
                Err(e) => return e,
            };

            unsafe {
                if !KERNEL.set_process_name(caller.data().pid, name) {
                    return -1;
                }
            }

            0
        },
    )?;

    linker.func_wrap(
        "env",
        "get_process_label",
        |caller: ProcessContext, buf_ptr: i32, buf_len: i32| -> i32 {
            let label = &caller.data().label;
            let bytes = label.as_bytes();
            if bytes.len() > buf_len as usize {
                return 0;
            }

            unsafe {
                KERNEL
                    .get_process_mut(caller.data().pid)
                    .unwrap()
                    .set_memory(buf_ptr as usize, bytes);
            }

            bytes.len() as i32
        },
    )?;

    linker.func_wrap(
        "env",
        "get_pid_by_name",
        |caller: ProcessContext, name_ptr: i32, name_len: i32| -> i32 {
            let name = match get_str(&caller, name_ptr, name_len) {
                Ok(n) => n,
                Err(_) => return 0,
            };

            unsafe { KERNEL.get_pid_by_name(name).as_i32() }
        },
    )?;

    // Filesystem

    linker.func_wrap(
        "env",
        "change_directory",
        |mut ctx: ProcessContext, path_ptr: i32, path_len: i32| -> i32 {
            let path = match get_str(&ctx, path_ptr, path_len) {
                Ok(p) => p,
                Err(e) => return e,
            };

            ctx.data_mut().change_directory(path)
        },
    )?;

    // Process

    linker.func_wrap_async("env", "yield_now", |_: ProcessContext, _: ()| {
        Box::new(async {
            yield_now().await;
        })
    })?;

    Ok(())
}
