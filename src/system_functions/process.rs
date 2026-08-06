use tokio::task::yield_now;

use crate::{
    async_file::AsyncFile,
    kernel::{Kernel, Pid, ProcessContext, ProcessLinker},
    ptr_cell::PtrCell,
    system_functions::{get_memory, get_str},
};

pub fn load_system_functions<T>(linker: &mut ProcessLinker<T>) -> wasmtime::Result<()> {
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
        |ctx: ProcessContext<T>,
         (path_ptr, path_len, argc, arg_lens, argv): (i32, u32, u32, i32, i32)| {
            let result = get_str(&ctx, path_ptr, path_len);
            let pid = ctx.data().pid;

            Box::new(async move {
                let path = match result {
                    Ok(p) => p,
                    Err(_) => return 0,
                };

                let arg_lens = get_memory(&ctx, arg_lens, argc * 4);
                let argv = get_memory(&ctx, argv, argc * 4);

                fn get_i32(slice: &[u8], i: usize) -> i32 {
                    i32::from_le_bytes(slice[(i * 4)..(i * 4 + 4)].try_into().unwrap())
                }

                let mut args = Vec::with_capacity(argc as usize);
                for i in 0..argc as usize {
                    let arg_len = get_i32(arg_lens, i);
                    let arg_ptr = get_i32(argv, i);
                    let arg = get_memory(&ctx, arg_ptr, arg_len as u32)
                        .to_owned()
                        .into_boxed_slice();
                    args.push(arg);
                }

                match Kernel::run_process(
                    ctx.data().kernel,
                    path,
                    pid,
                    args.into_boxed_slice(),
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

    linker.func_wrap("env", "get_argc", |mut ctx: ProcessContext<T>| -> u32 {
        ctx.data_mut().args.len() as u32
    })?;

    linker.func_wrap(
        "env",
        "prepare_arg",
        |mut ctx: ProcessContext<T>, index: u32| -> u32 {
            ctx.data_mut().prepare_arg(index as u16) as u32
        },
    )?;

    linker.func_wrap(
        "env",
        "prepare_var",
        |mut ctx: ProcessContext<T>, key_ptr: i32, key_len: u32| -> u32 {
            let key = get_memory(&ctx, key_ptr, key_len);
            ctx.data_mut().prepare_var(key) as u32
        },
    )?;

    linker.func_wrap(
        "env",
        "set_var",
        |mut ctx: ProcessContext<T>, key_ptr: i32, key_len: u32, value_ptr: i32, value_len: u32| {
            let key = get_memory(&ctx, key_ptr, key_len);
            let value = get_memory(&ctx, value_ptr, value_len);
            ctx.data_mut().set_var(key, value);
        },
    )?;

    linker.func_wrap(
        "env",
        "delte_var",
        |mut ctx: ProcessContext<T>, key_ptr: i32, key_len: u32| -> u32 {
            let key = get_memory(&ctx, key_ptr, key_len);
            ctx.data_mut().delete_var(key) as u32
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

    linker.func_wrap_async("env", "yield_now", |_: ProcessContext<T>, _: ()| {
        Box::new(async {
            yield_now().await;
        })
    })?;

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

    Ok(())
}
