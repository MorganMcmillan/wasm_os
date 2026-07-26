use std::io::{Write, stdout};
use std::time::Duration;

use tokio::task::yield_now;
use tokio::time::sleep;
use wasmtime::StoreContextMut;

use crate::KERNEL;
use crate::kernel::{Pid, ProcessContext, ProcessLinker};
use crate::process::Process;

#[allow(mismatched_lifetime_syntaxes)]
fn get_memory(ctx: *const ProcessContext, mem_ptr: i32, mem_len: i32) -> Result<&[u8], i32> {
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
        return Err(2);
    };
    Ok(string)
}

/// Loads all core system functions into the program.
/// TODO: allow drivers to register their own functions through this or a similar method.
pub fn load_system_functions(linker: &mut ProcessLinker) -> wasmtime::Result<()> {
    // Kernel methods

    linker.func_wrap(
        "debug_print",
        |ctx: StoreContextMut<Process>, (str_ptr, str_len): (i32, i32)| {
            if let Ok(bytes) = get_memory(&ctx, str_ptr, str_len) {
                // Is okay if it fails (although an error code would be nice)
                let _ = stdout().write_all(bytes);
            }
            Ok(())
        },
    )?;

    linker.func_wrap("get_pid", |ctx: ProcessContext, _: ()| {
        Ok((ctx.data().pid as i32,))
    })?;

    linker.func_wrap("get_parent_pid", |ctx: ProcessContext, _: ()| {
        Ok((ctx.data().parent_pid as i32,))
    })?;

    // Return Pid
    linker.func_wrap_async(
        "spawn",
        |ctx: ProcessContext, (path_ptr, path_len): (i32, i32)| {
            let result = get_str(&ctx, path_ptr, path_len);
            let pid = ctx.data().pid;

            Box::new(async move {
                unsafe {
                    let path = match result {
                        Ok(p) => p,
                        Err(_) => return Ok((0,)),
                    };

                    match KERNEL.run_process(path, pid).await {
                        Ok(id) => Ok((id as i32,)),
                        Err(_) => Ok((0,)),
                    }
                }
            })
        },
    )?;

    linker.func_wrap_async("exit", |mut caller: ProcessContext, (code,): (i32,)| {
        Box::new(async move {
            // Await join handle to end program execution.
            let _ = caller.data_mut().join_handle.as_mut().unwrap().await;
            caller.data_mut().exit_code = Some(code as u16);
            Ok(())
        })
    })?;

    // Time

    linker.func_wrap_async("sleep", |_: ProcessContext, (seconds,): (f64,)| {
        Box::new(async move {
            sleep(Duration::from_secs_f64(seconds)).await;
            Ok(())
        })
    })?;

    // Inter-process communication

    linker.func_wrap(
        "send_event",
        |caller: ProcessContext,
         (name_ptr, name_len, data_ptr, data_len, to_pid): (i32, i32, i32, i32, i32)| {
            let name = match get_str(&caller, name_ptr, name_len) {
                Ok(name) => name,
                Err(e) => return Ok((e,)),
            };

            let data = match get_memory(&caller, data_ptr, data_len) {
                Ok(d) => d,
                Err(e) => return Ok((e,)),
            };

            unsafe {
                KERNEL.send_event(name, data, caller.data().pid, to_pid as Pid);
            }

            Ok((0,))
        },
    )?;

    linker.func_wrap(
        "get_event_data",
        |caller: ProcessContext, (buf_ptr, buf_len): (i32, i32)| unsafe {
            let buf_ptr = buf_ptr as usize;
            let buf_len = buf_len as usize;

            let event = KERNEL.get_current_event();
            // WARNING: may cause an issue
            let process = KERNEL.get_process_mut(caller.data().pid).unwrap();
            if event.data.len() < buf_len {
                process.set_memory(buf_ptr, &event.data);
            } else {
                process.set_memory(buf_ptr, &event.data[..buf_len]);
            }
            Ok(())
        },
    )?;

    linker.func_wrap("get_event_sender", |_: ProcessContext, _: ()| {
        let event = unsafe { KERNEL.get_current_event() };
        Ok((event.sent_by_pid as i32,))
    })?;

    linker.func_wrap(
        "add_event_handler",
        |mut ctx: ProcessContext, (name_ptr, name_len): (i32, i32)| {
            let name = match get_str(&ctx, name_ptr, name_len) {
                Ok(n) => n,
                Err(e) => return Ok((e,)),
            };
            let interned_name = unsafe { KERNEL.intern_event_name(name) };

            let handler = unsafe {
                KERNEL
                    .get_process(ctx.data().pid)
                    .unwrap()
                    .instance
                    .get_typed_func::<(i32,), ()>(&mut ctx, name)
                    .unwrap()
            };

            ctx.data_mut().add_event_handler(interned_name, handler);
            Ok((0,))
        },
    )?;

    linker.func_wrap(
        "remove_event_handler",
        |mut caller: ProcessContext, (name_ptr, name_len): (i32, i32)| {
            let name = match get_str(&caller, name_ptr, name_len) {
                Ok(o) => o,
                Err(e) => return Ok((e,)),
            };
            let interned_name = unsafe { KERNEL.intern_event_name(name) };

            caller.data_mut().remove_event_handler(interned_name);
            Ok((0,))
        },
    )?;

    linker.func_wrap(
        "proc_memcpy",
        |caller: ProcessContext, (src_pid, src, dest, len): (i32, i32, i32, i32)| {
            let src_pid = src_pid as Pid;
            let src = src as usize;
            let dest = dest as usize;
            let len = len as usize;

            unsafe {
                let Some(src_proc) = KERNEL.get_process(src_pid) else {
                    return Ok((-1,));
                };

                KERNEL
                    .get_process_mut(caller.data().pid)
                    .unwrap()
                    .set_memory(dest, src_proc.get_memory(src, len));
            }
            Ok((0,))
        },
    )?;

    linker.func_wrap(
        "set_process_name",
        |caller: ProcessContext, (name_ptr, name_len): (i32, i32)| {
            let name = match get_str(&caller, name_ptr, name_len) {
                Ok(n) => n,
                Err(e) => return Ok((e,)),
            };

            unsafe {
                if !KERNEL.set_process_name(caller.data().pid, name) {
                    return Ok((-1,));
                }
            }

            Ok((0,))
        },
    )?;

    linker.func_wrap(
        "get_process_label",
        |caller: ProcessContext, (buf_ptr, buf_len): (i32, i32)| {
            let label = &caller.data().label;
            let bytes = label.as_bytes();
            if bytes.len() > buf_len as usize {
                return Ok((0,));
            }

            unsafe {
                KERNEL
                    .get_process_mut(caller.data().pid)
                    .unwrap()
                    .set_memory(buf_ptr as usize, bytes);
            }

            Ok((bytes.len() as i32,))
        },
    )?;

    linker.func_wrap(
        "get_pid_by_name",
        |caller: ProcessContext, (name_ptr, name_len): (i32, i32)| {
            let name = match get_str(&caller, name_ptr, name_len) {
                Ok(n) => n,
                Err(_) => return Ok((0,)),
            };

            let pid = unsafe { KERNEL.get_pid_by_name(name) as i32 };
            Ok((pid,))
        },
    )?;

    // Process

    linker.func_wrap_async("yield_now", |_: ProcessContext, _: ()| {
        Box::new(async {
            yield_now().await;
            Ok(())
        })
    })?;

    Ok(())
}
