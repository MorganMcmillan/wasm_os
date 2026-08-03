use std::io::{Write, stdout};
use std::time::Duration;

use string_interner::Symbol;
use string_interner::symbol::SymbolU32;
use tokio::task::yield_now;
use tokio::time::sleep;
use wasmtime::Linker;

use crate::async_file::AsyncFile;
use crate::id::Id;
use crate::kernel::{Kernel, Pid, ProcessContext};
use crate::process::Process;
use crate::ptr_cell::PtrCell;

const EVENT_HANDLER_NOT_FOUND: &str = "Could not get event handler.\nIt's possible your program was not compiled with wasm-ld having the `--export-table` flag.";

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
    // Kernel methods

    linker.func_wrap(
        "env",
        "debug_print",
        |ctx: ProcessContext<T>, str_ptr: i32, str_len: u32| {
            let _ = stdout().write_all(get_memory(&ctx, str_ptr, str_len));
        },
    )?;

    linker.func_wrap("env", "get_pid", |ctx: ProcessContext<T>| -> i32 {
        ctx.data().pid.as_i32()
    })?;

    linker.func_wrap("env", "get_parent_pid", |ctx: ProcessContext<T>| -> i32 {
        ctx.data().parent_pid.as_i32()
    })?;

    linker.func_wrap("env", "iter_children", |mut ctx: ProcessContext<T>| {
        ctx.data_mut().iter_children();
    })?;

    linker.func_wrap("env", "next_child", |mut ctx: ProcessContext<T>| -> i32 {
        ctx.data_mut().next_child().as_i32()
    })?;

    // Return Pid
    linker.func_wrap_async(
        "env",
        "spawn",
        |ctx: ProcessContext<T>, (path_ptr, path_len): (i32, u32)| {
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

    linker.func_wrap_async(
        "env",
        "exit",
        |mut ctx: ProcessContext<T>, (code,): (i32,)| {
            Box::new(async move {
                // Await join handle to end program execution.
                ctx.data_mut().kill().await;
                ctx.data_mut().exit_code = Some(code as u16);
            })
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

    linker.func_wrap_async("env", "yield_now", |_: ProcessContext<T>, _: ()| {
        Box::new(async {
            yield_now().await;
        })
    })?;

    // Inter-process communication

    linker.func_wrap(
        "env",
        "send_event",
        |ctx: ProcessContext<T>,
         name_ptr: i32,
         name_len: u32,
         data_ptr: i32,
         data_len: u32,
         to_pid: i32|
         -> i32 {
            let name = match get_str(&ctx, name_ptr, name_len) {
                Ok(name) => name,
                Err(e) => return e,
            };

            let data = get_memory(&ctx, data_ptr, data_len);

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
        |ctx: ProcessContext<T>, to_pid: i32| -> i32 {
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
        |ctx: ProcessContext<T>, buf_ptr: i32, buf_len: i32| {
            let buf_ptr = buf_ptr as usize;
            let buf_len = buf_len as usize;

            let event = ctx.data().kernel.get_current_event();
            let mut data = event.data();
            if data.len() < buf_len {
                data = &data[..buf_len]
            }

            ctx.data().set_memory(buf_ptr, data);
        },
    )?;

    linker.func_wrap("env", "get_event_sender", |ctx: ProcessContext<T>| -> i32 {
        let event = ctx.data().kernel.get_current_event();
        event.sent_by_pid.as_i32()
    })?;

    linker.func_wrap(
        "env",
        "add_event_handler",
        |mut ctx: ProcessContext<T>, name_ptr: i32, name_len: u32, handler_index: i32| -> i32 {
            let name = match get_str(&ctx, name_ptr, name_len) {
                Ok(n) => n,
                Err(e) => return e,
            };
            let interned_name = ctx.data().kernel.borrow_static().intern_event_name(name);

            let handler = ctx
                .data()
                .as_wasm_process()
                .get_exported_function(handler_index)
                .expect(EVENT_HANDLER_NOT_FOUND)
                .typed::<i32, ()>(&ctx)
                .unwrap();

            ctx.data_mut().add_event_handler(interned_name, handler);
            0
        },
    )?;

    linker.func_wrap(
        "env",
        "remove_event_handler",
        |mut ctx: ProcessContext<T>, name_ptr: i32, name_len: u32| -> i32 {
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
        "set_default_handler",
        |mut ctx: ProcessContext<T>, handler_index: i32| -> i32 {
            let handler = ctx
                .data()
                .as_wasm_process()
                .get_exported_function(handler_index)
                .expect(EVENT_HANDLER_NOT_FOUND)
                .typed::<(i32, i32), ()>(&ctx)
                .unwrap();

            ctx.data_mut().set_default_handler(handler);
            0
        },
    )?;

    linker.func_wrap(
        "env",
        "prepare_event_name",
        |mut ctx: ProcessContext<T>, symbol: u32| -> i32 {
            let Some(symbol) = SymbolU32::try_from_usize(symbol as usize) else {
                return -1;
            };

            let event_name = ctx.data().kernel.get_event_name(symbol);
            ctx.data_mut().set_data(event_name.as_bytes()) as i32
        },
    )?;

    linker.func_wrap(
        "env",
        "copy_process_memory",
        |mut ctx: ProcessContext<T>, src_pid: i32, src: i32, dest: i32, len: u32| -> i32 {
            let src_pid = Pid::from_i32(src_pid);
            let dest = dest as usize;

            let Some(src_proc) = ctx
                .data_mut()
                .kernel
                .borrow_static()
                .get_process_mut(src_pid)
            else {
                return 1;
            };

            ctx.data()
                .set_memory(dest, src_proc.get_memory(src as usize, len as usize));
            0
        },
    )?;

    linker.func_wrap(
        "env",
        "set_process_name",
        |ctx: ProcessContext<T>, name_ptr: i32, name_len: u32| -> i32 {
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
        "prepare_process_label",
        |mut ctx: ProcessContext<T>| -> i32 {
            let mut ctx_cell = PtrCell::new(&mut ctx);
            let label = ctx.data().label.as_bytes();
            ctx_cell.get_mut().data_mut().set_data(label) as i32
        },
    )?;

    linker.func_wrap(
        "env",
        "get_pid_by_name",
        |ctx: ProcessContext<T>, name_ptr: i32, name_len: u32| -> i32 {
            let name = match get_str(&ctx, name_ptr, name_len) {
                Ok(n) => n,
                Err(_) => return 0,
            };

            ctx.data().kernel.get_pid_by_name(name).as_i32()
        },
    )?;

    // Data

    linker.func_wrap("env", "get_data_length", |ctx: ProcessContext<T>| -> u32 {
        ctx.data().byte_data.len() as u32
    })?;

    linker.func_wrap(
        "env",
        "read_data",
        |ctx: ProcessContext<T>, buf_ptr: i32, buf_len: u32| {
            let mut buf = get_memory(&ctx, buf_ptr, buf_len);
            let _ = buf.write_all(&ctx.data().byte_data);
        },
    )?;

    // Filesystem

    linker.func_wrap(
        "env",
        "is_directory",
        |ctx: ProcessContext<T>, path_ptr: i32, path_len: u32| -> i32 {
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
        |ctx: ProcessContext<T>, path_ptr: i32, path_len: u32| -> i32 {
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
        |ctx: ProcessContext<T>, path_ptr: i32, path_len: u32| -> i32 {
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
        |ctx: ProcessContext<T>, path_ptr: i32, path_len: u32| -> i32 {
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
        |ctx: ProcessContext<T>, path_ptr: i32, path_len: u32| -> i64 {
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
        |ctx: ProcessContext<T>, path_ptr: i32, path_len: u32| -> i64 {
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
        |ctx: ProcessContext<T>, path_ptr: i32, path_len: u32| -> i64 {
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
        |mut ctx: ProcessContext<T>, path_ptr: i32, path_len: u32, mode: i32| {
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
        |mut ctx: ProcessContext<T>, (fd, buf_ptr, buf_len): (i32, i32, u32)| {
            Box::new(async move {
                let buf = get_memory(&ctx, buf_ptr, buf_len);

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
        "prepare_file_contents",
        |mut ctx: ProcessContext<T>, (fd,): (i32,)| {
            Box::new(async move {
                let Some(file) = ctx.data_mut().get_file(Id::from_i32(fd)) else {
                    return -2;
                };

                let mut contents = Vec::with_capacity(64);

                if file.read_to_end(&mut contents).await.is_err() {
                    return -1;
                };

                ctx.data_mut().set_data(&contents) as i32
            })
        },
    )?;

    linker.func_wrap_async(
        "env",
        "prepare_path_contents",
        |mut ctx: ProcessContext<T>, (path_ptr, path_len): (i32, u32)| {
            Box::new(async move {
                let path = match get_str(&ctx, path_ptr, path_len) {
                    Ok(p) => p,
                    Err(e) => return e,
                };

                let contents = match ctx.data_mut().read_entire_file(path).await {
                    Ok(c) => c,
                    Err(_) => return -1,
                };

                ctx.data_mut().set_data(&contents) as i32
            })
        },
    )?;

    linker.func_wrap_async(
        "env",
        "write_file",
        |mut ctx: ProcessContext<T>, (fd, src_ptr, src_len): (i32, i32, u32)| {
            Box::new(async move {
                let src = get_memory(&ctx, src_ptr, src_len);

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
        |mut ctx: ProcessContext<T>, (fd, offset, from): (i32, i32, i32)| {
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
        |mut ctx: ProcessContext<T>, fd: i32| -> i32 {
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
        |mut ctx: ProcessContext<T>, path_ptr: i32, path_len: u32| -> i32 {
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
        |mut ctx: ProcessContext<T>,
         from_ptr: i32,
         from_len: u32,
         to_ptr: i32,
         to_len: u32|
         -> i32 {
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
        |mut ctx: ProcessContext<T>,
         from_ptr: i32,
         from_len: u32,
         to_ptr: i32,
         to_len: u32|
         -> i32 {
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
        |mut ctx: ProcessContext<T>, path_ptr: i32, path_len: u32| -> i32 {
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
        |mut ctx: ProcessContext<T>, path_ptr: i32, path_len: u32| -> i32 {
            let path = match get_str(&ctx, path_ptr, path_len) {
                Ok(f) => f,
                Err(e) => return e,
            };

            ctx.data_mut().create_directory(path)
        },
    )?;

    Ok(())
}
