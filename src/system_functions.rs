use std::io::{Write, stdout};
use std::time::Duration;

use tokio::task::yield_now;
use tokio::time::sleep;
use wasmtime::Linker;

use crate::async_file::AsyncFile;
use crate::id::Id;
use crate::kernel::{Kernel, Pid, ProcessContext};
use crate::process::Process;

#[allow(mismatched_lifetime_syntaxes)]
pub fn get_memory(ctx: &ProcessContext, mem_ptr: i32, mem_len: i32) -> Result<&'static [u8], i32> {
    let process = ctx
        .data()
        .kernel
        .borrow_static()
        .get_process_mut(ctx.data().pid)
        .unwrap();

    let mem = process.get_memory(mem_ptr as usize, mem_len as usize);
    Ok(mem)
}

#[allow(mismatched_lifetime_syntaxes)]
pub fn get_memory_mut(
    ctx: &ProcessContext,
    mem_ptr: i32,
    mem_len: i32,
) -> Result<&'static mut [u8], i32> {
    let process = ctx
        .data()
        .kernel
        .borrow_static()
        .get_process_mut(ctx.data().pid)
        .unwrap();

    let mem = process.get_memory_mut(mem_ptr as usize, mem_len as usize);
    Ok(mem)
}

#[allow(mismatched_lifetime_syntaxes)]
fn get_str(ctx: &ProcessContext, str_ptr: i32, str_len: i32) -> Result<&'static str, i32> {
    let string = get_memory(ctx, str_ptr, str_len)?;
    let Ok(string) = str::from_utf8(string) else {
        return Err(-2);
    };
    Ok(string)
}

/// Loads all core system functions into the program.
pub fn load_system_functions(linker: &mut Linker<Process>) -> wasmtime::Result<()> {
    // Kernel methods

    linker.func_wrap(
        "env",
        "debug_print",
        |ctx: ProcessContext, str_ptr: i32, str_len: i32| {
            if let Ok(bytes) = get_memory(&ctx, str_ptr, str_len) {
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
                let path = match result {
                    Ok(p) => p,
                    Err(_) => return 0,
                };

                match Kernel::run_process(
                    ctx.data().kernel,
                    path,
                    pid,
                    AsyncFile::Null,
                    AsyncFile::Null,
                    AsyncFile::Null,
                )
                .await
                {
                    Ok(id) => id.as_i32(),
                    Err(_) => 0,
                }
            })
        },
    )?;

    linker.func_wrap_async("env", "exit", |mut ctx: ProcessContext, (code,): (i32,)| {
        Box::new(async move {
            // Await join handle to end program execution.
            ctx.data_mut().kill().await;
            ctx.data_mut().exit_code = Some(code as u16);
        })
    })?;

    // Time

    linker.func_wrap_async("env", "sleep", |_: ProcessContext, (seconds,): (f64,)| {
        Box::new(async move {
            sleep(Duration::from_secs_f64(seconds)).await;
        })
    })?;

    linker.func_wrap_async("env", "yield_now", |_: ProcessContext, _: ()| {
        Box::new(async {
            yield_now().await;
        })
    })?;

    // Inter-process communication

    linker.func_wrap(
        "env",
        "send_event",
        |ctx: ProcessContext,
         name_ptr: i32,
         name_len: i32,
         data_ptr: i32,
         data_len: i32,
         to_pid: i32|
         -> i32 {
            let name = match get_str(&ctx, name_ptr, name_len) {
                Ok(name) => name,
                Err(e) => return e,
            };

            let data = match get_memory(&ctx, data_ptr, data_len) {
                Ok(d) => d,
                Err(e) => return e,
            };

            ctx.data().kernel.borrow_static().send_event(
                name,
                data,
                ctx.data().pid,
                Pid::from_i32(to_pid),
            )
        },
    )?;

    linker.func_wrap(
        "env",
        "resend_event",
        |ctx: ProcessContext, to_pid: i32| -> i32 {
            let event = ctx.data().kernel.get_current_event();
            let pid = ctx.data().pid;
            ctx.data()
                .kernel
                .borrow_static()
                .resend_event(event, pid, Pid::from_i32(to_pid))
        },
    )?;

    linker.func_wrap(
        "env",
        "get_event_data",
        |ctx: ProcessContext, buf_ptr: i32, buf_len: i32| {
            let buf_ptr = buf_ptr as usize;
            let buf_len = buf_len as usize;

            let event = ctx.data().kernel.get_current_event();
            let mut data = event.data();
            if data.len() < buf_len {
                data = &data[..buf_len]
            }
            let process = ctx
                .data()
                .kernel
                .borrow_static()
                .get_process_mut(ctx.data().pid)
                .unwrap();

            process.set_memory(buf_ptr, data);
        },
    )?;

    linker.func_wrap("env", "get_event_sender", |ctx: ProcessContext| -> i32 {
        let event = ctx.data().kernel.get_current_event();
        event.sent_by_pid.as_i32()
    })?;

    linker.func_wrap(
        "env",
        "add_event_handler",
        |mut ctx: ProcessContext, name_ptr: i32, name_len: i32| -> i32 {
            let name = match get_str(&ctx, name_ptr, name_len) {
                Ok(n) => n,
                Err(e) => return e,
            };
            let interned_name = ctx.data().kernel.borrow_static().intern_event_name(name);

            let handler = ctx
                .get_export(name)
                .unwrap()
                .into_func()
                .unwrap()
                .typed::<i32, ()>(&ctx)
                .unwrap();

            ctx.data_mut().add_event_handler(interned_name, handler);
            0
        },
    )?;

    linker.func_wrap(
        "env",
        "remove_event_handler",
        |mut ctx: ProcessContext, name_ptr: i32, name_len: i32| -> i32 {
            let name = match get_str(&ctx, name_ptr, name_len) {
                Ok(o) => o,
                Err(e) => return e,
            };
            let interned_name = ctx.data().kernel.borrow_static().intern_event_name(name);

            ctx.data_mut().remove_event_handler(interned_name);
            0
        },
    )?;

    linker.func_wrap(
        "env",
        "proc_memcpy",
        |ctx: ProcessContext, src_pid: i32, src: i32, dest: i32, len: i32| -> i32 {
            let src_pid = Pid::from_i32(src_pid);
            let src = src as usize;
            let dest = dest as usize;
            let len = len as usize;

            let Some(src_proc) = ctx.data().kernel.get_process(src_pid) else {
                return 1;
            };

            ctx.data()
                .kernel
                .borrow_static()
                .get_process_mut(ctx.data().pid)
                .unwrap()
                .set_memory(dest, src_proc.get_memory(src, len));
            0
        },
    )?;

    linker.func_wrap(
        "env",
        "set_process_name",
        |ctx: ProcessContext, name_ptr: i32, name_len: i32| -> i32 {
            let name = match get_str(&ctx, name_ptr, name_len) {
                Ok(n) => n,
                Err(e) => return e,
            };

            if !ctx
                .data()
                .kernel
                .borrow_static()
                .set_process_name(ctx.data().pid, name)
            {
                return -1;
            }

            0
        },
    )?;

    linker.func_wrap(
        "env",
        "get_process_label",
        |ctx: ProcessContext, buf_ptr: i32, buf_len: i32| -> i32 {
            let label = &ctx.data().label;
            let bytes = label.as_bytes();
            if bytes.len() > buf_len as usize {
                return 0;
            }

            ctx.data()
                .kernel
                .borrow_static()
                .get_process_mut(ctx.data().pid)
                .unwrap()
                .set_memory(buf_ptr as usize, bytes);

            bytes.len() as i32
        },
    )?;

    linker.func_wrap(
        "env",
        "get_pid_by_name",
        |ctx: ProcessContext, name_ptr: i32, name_len: i32| -> i32 {
            let name = match get_str(&ctx, name_ptr, name_len) {
                Ok(n) => n,
                Err(_) => return 0,
            };

            ctx.data().kernel.get_pid_by_name(name).as_i32()
        },
    )?;

    // Filesystem

    linker.func_wrap(
        "env",
        "is_directory",
        |ctx: ProcessContext, path_ptr: i32, path_len: i32| -> i32 {
            let path = match get_str(&ctx, path_ptr, path_len) {
                Ok(p) => p,
                Err(_) => return 0,
            };

            ctx.data().is_directory(path) as i32
        },
    )?;

    linker.func_wrap(
        "env",
        "is_file",
        |ctx: ProcessContext, path_ptr: i32, path_len: i32| -> i32 {
            let path = match get_str(&ctx, path_ptr, path_len) {
                Ok(p) => p,
                Err(_) => return 0,
            };

            ctx.data().is_file(path) as i32
        },
    )?;

    linker.func_wrap(
        "env",
        "file_exists",
        |ctx: ProcessContext, path_ptr: i32, path_len: i32| -> i32 {
            let path = match get_str(&ctx, path_ptr, path_len) {
                Ok(p) => p,
                Err(_) => return 0,
            };

            ctx.data().file_exists(path) as i32
        },
    )?;

    linker.func_wrap(
        "env",
        "file_size",
        |ctx: ProcessContext, path_ptr: i32, path_len: i32| -> i32 {
            let path = match get_str(&ctx, path_ptr, path_len) {
                Ok(p) => p,
                Err(_) => return -1,
            };

            ctx.data().file_size(path)
        },
    )?;

    linker.func_wrap(
        "env",
        "file_created",
        |ctx: ProcessContext, path_ptr: i32, path_len: i32| -> i64 {
            let path = match get_str(&ctx, path_ptr, path_len) {
                Ok(p) => p,
                Err(_) => return -1,
            };

            ctx.data().file_created(path)
        },
    )?;

    linker.func_wrap(
        "env",
        "file_accessed",
        |ctx: ProcessContext, path_ptr: i32, path_len: i32| -> i64 {
            let path = match get_str(&ctx, path_ptr, path_len) {
                Ok(p) => p,
                Err(_) => return -1,
            };

            ctx.data().file_accessed(path)
        },
    )?;
    linker.func_wrap(
        "env",
        "file_modified",
        |ctx: ProcessContext, path_ptr: i32, path_len: i32| -> i64 {
            let path = match get_str(&ctx, path_ptr, path_len) {
                Ok(p) => p,
                Err(_) => return -1,
            };

            ctx.data().file_modified(path)
        },
    )?;

    linker.func_wrap(
        "env",
        "open_file",
        |mut ctx: ProcessContext, path_ptr: i32, path_len: i32, mode: i32| {
            let path = match get_str(&ctx, path_ptr, path_len) {
                Ok(p) => p,
                Err(_) => return 0,
            };

            ctx.data_mut().open_file(path, mode).as_i32()
        },
    )?;

    linker.func_wrap_async(
        "env",
        "read_file",
        |mut ctx: ProcessContext, (fd, buf_ptr, buf_len): (i32, i32, i32)| {
            Box::new(async move {
                let buf = match get_memory_mut(&ctx, buf_ptr, buf_len) {
                    Ok(b) => b,
                    Err(e) => return e,
                };

                let Some(file) = ctx.data_mut().get_file(Id::from_i32(fd)) else {
                    return -2;
                };

                match file.read(buf).await {
                    Ok(bytes) => bytes as i32,
                    Err(_) => -1,
                }
            })
        },
    )?;

    linker.func_wrap_async(
        "env",
        "write_file",
        |mut ctx: ProcessContext, (fd, src_ptr, src_len): (i32, i32, i32)| {
            Box::new(async move {
                let src = match get_memory(&ctx, src_ptr, src_len) {
                    Ok(s) => s,
                    Err(e) => return e,
                };

                let Some(file) = ctx.data_mut().get_file(Id::from_i32(fd)) else {
                    return -2;
                };

                match file.write(src).await {
                    Ok(bytes) => bytes as i32,
                    Err(_) => -1,
                }
            })
        },
    )?;

    linker.func_wrap_async(
        "env",
        "seek",
        |mut ctx: ProcessContext, (fd, offset, from): (i32, i32, i32)| {
            Box::new(async move {
                let Some(file) = ctx.data_mut().get_file(Id::from_i32(fd)) else {
                    return -2;
                };

                match file.seek(offset as i64, from as u8).await {
                    Ok(new_offset) => new_offset as i32,
                    Err(_) => -1,
                }
            })
        },
    )?;

    linker.func_wrap(
        "env",
        "close_file",
        |mut ctx: ProcessContext, fd: i32| -> i32 {
            if ctx.data_mut().open_files.delete_id(Id::from_i32(fd)) {
                0
            } else {
                -1
            }
        },
    )?;

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

    linker.func_wrap(
        "env",
        "move_file",
        |mut ctx: ProcessContext, from_ptr: i32, from_len: i32, to_ptr: i32, to_len: i32| -> i32 {
            let from = match get_str(&ctx, from_ptr, from_len) {
                Ok(f) => f,
                Err(e) => return e,
            };
            let to = match get_str(&ctx, to_ptr, to_len) {
                Ok(t) => t,
                Err(e) => return e,
            };

            ctx.data_mut().move_file(from, to)
        },
    )?;

    linker.func_wrap(
        "env",
        "copy_file",
        |mut ctx: ProcessContext, from_ptr: i32, from_len: i32, to_ptr: i32, to_len: i32| -> i32 {
            let from = match get_str(&ctx, from_ptr, from_len) {
                Ok(f) => f,
                Err(e) => return e,
            };
            let to = match get_str(&ctx, to_ptr, to_len) {
                Ok(t) => t,
                Err(e) => return e,
            };

            ctx.data_mut().copy_file(from, to)
        },
    )?;

    linker.func_wrap(
        "env",
        "delete_file",
        |mut ctx: ProcessContext, path_ptr: i32, path_len: i32| -> i32 {
            let path = match get_str(&ctx, path_ptr, path_len) {
                Ok(f) => f,
                Err(e) => return e,
            };

            ctx.data_mut().delete_file(path)
        },
    )?;

    linker.func_wrap(
        "env",
        "create_directory",
        |mut ctx: ProcessContext, path_ptr: i32, path_len: i32| -> i32 {
            let path = match get_str(&ctx, path_ptr, path_len) {
                Ok(f) => f,
                Err(e) => return e,
            };

            ctx.data_mut().create_directory(path)
        },
    )?;

    Ok(())
}
