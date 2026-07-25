use std::io::{Write, stdout};
use std::time::Duration;

use tokio::task::yield_now;
use tokio::time::sleep;
use wasmtime::{Caller, Linker};

use crate::KERNEL;
use crate::kernel::Pid;
use crate::process::Process;

pub type ProcessCaller<'a> = Caller<'a, Process>;

#[allow(mismatched_lifetime_syntaxes)]
fn get_memory(caller: *const ProcessCaller, mem_ptr: i32, mem_len: i32) -> Result<&[u8], i32> {
    unsafe {
        let process = KERNEL.get_process_mut((*caller).data().pid).unwrap();

        let mem = process.get_memory(mem_ptr as usize, mem_len as usize);
        Ok(mem)
    }
}

#[allow(mismatched_lifetime_syntaxes)]
fn get_str(caller: *const ProcessCaller, str_ptr: i32, str_len: i32) -> Result<&str, i32> {
    let string = get_memory(caller, str_ptr, str_len)?;
    let Ok(string) = str::from_utf8(string) else {
        return Err(2);
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
        |caller: ProcessCaller, str_ptr: i32, str_len: i32| {
            if let Ok(bytes) = get_memory(&caller, str_ptr, str_len) {
                // Is okay if it fails (although an error code would be nice)
                let _ = stdout().write_all(bytes);
            }
        },
    )?;

    linker.func_wrap("env", "get_pid", |caller: ProcessCaller| -> i32 {
        caller.data().pid as i32
    })?;

    linker.func_wrap("env", "get_parent_pid", |caller: ProcessCaller| -> i32 {
        caller.data().parent_pid as i32
    })?;

    // Return Pid
    linker.func_wrap_async(
        "env",
        "spawn",
        |caller: ProcessCaller, (path_ptr, path_len): (i32, i32)| {
            let result = get_str(&caller, path_ptr, path_len);
            let pid = caller.data().pid;

            Box::new(async move {
                unsafe {
                    let path = match result {
                        Ok(p) => p,
                        Err(_) => return 0,
                    };

                    match KERNEL.run_process(path, pid).await {
                        Ok(id) => id as i32,
                        Err(_) => 0,
                    }
                }
            })
        },
    )?;

    linker.func_wrap_async(
        "env",
        "exit",
        |mut caller: ProcessCaller, (code,): (i32,)| {
            Box::new(async move {
                // Await join handle to end program execution.
                let _ = caller.data_mut().join_handle.as_mut().unwrap().await;
                caller.data_mut().exit_code = Some(code as u16);
            })
        },
    )?;

    // Time

    linker.func_wrap_async("env", "sleep", |_: ProcessCaller, (seconds,): (f64,)| {
        Box::new(async move {
            sleep(Duration::from_secs_f64(seconds)).await;
        })
    })?;

    // Inter-process communication

    linker.func_wrap(
        "env",
        "send_event",
        |caller: ProcessCaller,
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

            unsafe {
                KERNEL.send_event(name, data, caller.data().pid, to_pid as Pid);
            }

            0
        },
    )?;

    linker.func_wrap(
        "env",
        "get_event_data",
        |caller: ProcessCaller, buf_ptr: i32, buf_len: i32| unsafe {
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
        },
    )?;

    linker.func_wrap("env", "get_event_sender", |_: ProcessCaller| -> i32 {
        let event = unsafe { KERNEL.get_current_event() };
        event.sent_by_pid as i32
    })?;

    linker.func_wrap(
        "env",
        "add_event_handler",
        |mut caller: ProcessCaller, name_ptr: i32, name_len: i32| -> i32 {
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
                .typed::<(i32,), ()>(&caller)
                .unwrap();

            caller.data_mut().add_event_handler(interned_name, handler);
            0
        },
    )?;

    linker.func_wrap(
        "env",
        "remove_event_handler",
        |mut caller: ProcessCaller, name_ptr: i32, name_len: i32| -> i32 {
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
        |caller: ProcessCaller, src_pid: i32, src: i32, dest: i32, len: i32| -> i32 {
            let src_pid = src_pid as Pid;
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
        |caller: ProcessCaller, name_ptr: i32, name_len: i32| -> i32 {
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
        |caller: ProcessCaller, buf_ptr: i32, buf_len: i32| -> i32 {
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
        |caller: ProcessCaller, name_ptr: i32, name_len: i32| -> i32 {
            let name = match get_str(&caller, name_ptr, name_len) {
                Ok(n) => n,
                Err(_) => return 0,
            };

            unsafe { KERNEL.get_pid_by_name(name) as i32 }
        },
    )?;

    // Process

    linker.func_wrap_async("env", "yield_now", |_: ProcessCaller, _: ()| {
        Box::new(async {
            yield_now().await;
        })
    })?;

    Ok(())
}
