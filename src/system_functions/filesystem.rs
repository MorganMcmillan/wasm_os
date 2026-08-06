use std::os::unix::ffi::OsStrExt as _;

use crate::{
    cell::ptr_cell::PtrCell,
    id::Id,
    kernel::{ProcessContext, ProcessLinker},
    process::Process,
    system_functions::{get_memory, get_str},
};

pub fn load_system_functions<T>(linker: &mut ProcessLinker<T>) -> wasmtime::Result<()> {
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

                ctx.data_mut().prepare_bytes(&contents) as i32
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

                ctx.data_mut().prepare_bytes(&contents) as i32
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
        "write_path",
        |mut ctx: ProcessContext<T>,
         (path_ptr, path_len, src_ptr, src_len): (i32, u32, i32, u32)| {
            Box::new(async move {
                let path = match get_str(&ctx, path_ptr, path_len) {
                    Ok(p) => p,
                    Err(e) => return e,
                };

                let src = get_memory(&ctx, src_ptr, src_len);

                if ctx.data_mut().write_entire_file(path, src).await.is_ok() {
                    0
                } else {
                    -1
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

    linker.func_wrap(
        "env",
        "read_dir",
        |mut ctx: ProcessContext<T>, path_ptr: i32, path_len: u32| -> i32 {
            let path = match get_str(&ctx, path_ptr, path_len) {
                Ok(f) => f,
                Err(e) => return e,
            };

            ctx.data_mut().iter_directory(path).as_i32()
        },
    )?;

    linker.func_wrap(
        "env",
        "prepare_dir_entry",
        |mut ctx: ProcessContext<T>, dir_id: i32| -> i32 {
            let mut ctx_cell = PtrCell::new(&mut ctx);
            let dir_id = Id::from_i32(dir_id);
            let Some(dir_iter) = ctx_cell
                .get_mut()
                .data_mut()
                .directory_iterators
                .data_mut(dir_id)
            else {
                return 0;
            };

            fn try_next<T>(
                process: &mut Process<T>,
                dir_iter: &mut cap_std::fs::ReadDir,
                dir_id: Id,
            ) -> i32 {
                match dir_iter.next() {
                    Some(Ok(entry)) => process.prepare_bytes(entry.file_name().as_bytes()) as i32,
                    Some(Err(_)) => try_next(process, dir_iter, dir_id),
                    None => {
                        process.directory_iterators.delete_id(dir_id);
                        0
                    }
                }
            }

            try_next(ctx.data_mut(), dir_iter, dir_id)
        },
    )?;

    linker.func_wrap(
        "env",
        "close_dir",
        |mut ctx: ProcessContext<T>, dir_id: i32| {
            ctx.data_mut()
                .directory_iterators
                .delete_id(Id::from_i32(dir_id));
        },
    )?;

    Ok(())
}
