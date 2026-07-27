use std::time::Duration;

use tokio::task::yield_now;
use tokio::time::sleep;
use wasmtime::StoreContextMut;
use wasmtime::component::{WasmList, WasmStr};

use crate::KERNEL;
use crate::kernel::{Pid, ProcessContext, ProcessLinker};
use crate::process::Process;

/// Loads all core system functions into the program.
/// TODO: allow drivers to register their own functions through this or a similar method.
pub fn load_system_functions(linker: &mut ProcessLinker) -> wasmtime::Result<()> {
    // Kernel methods

    linker.func_wrap(
        "debug-print",
        |ctx: StoreContextMut<Process>, (contents,): (WasmStr,)| {
            if let Ok(contents) = contents.to_str(&ctx) {
                println!("{contents}");
            }
            Ok(())
        },
    )?;

    linker.func_wrap("get-pid", |ctx: ProcessContext, _: ()| {
        Ok((ctx.data().pid as i32,))
    })?;

    linker.func_wrap("get-parent-pid", |ctx: ProcessContext, _: ()| {
        Ok((ctx.data().parent_pid as i32,))
    })?;

    // Return Pid
    linker.func_wrap_async("spawn", |ctx: ProcessContext, (path,): (WasmStr,)| {
        let result = match path.to_str(&ctx) {
            Ok(p) => Ok(p.as_ref().to_owned()),
            Err(_) => Err((0,)),
        };
        let pid = ctx.data().pid;

        Box::new(async move {
            unsafe {
                let path = match result {
                    Ok(p) => p,
                    Err(e) => return Ok(e),
                };

                match KERNEL.run_process(path.as_ref(), pid).await {
                    Ok(id) => Ok((id as i32,)),
                    Err(_) => Ok((0,)),
                }
            }
        })
    })?;

    linker.func_wrap_async("exit", |mut ctx: ProcessContext, (code,): (i32,)| {
        Box::new(async move {
            // Await join handle to end program execution.
            let _ = ctx.data_mut().join_handle.as_mut().unwrap().await;
            ctx.data_mut().exit_code = Some(code as u16);
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
        "send-event",
        |ctx: ProcessContext, (name, data, to_pid): (WasmStr, WasmList<u8>, i32)| {
            let name = match name.to_str(&ctx) {
                Ok(name) => name,
                Err(_) => return Ok((-1,)),
            };

            let data = data.as_le_slice(&ctx);

            unsafe {
                KERNEL.send_event(name.as_ref(), data, ctx.data().pid, to_pid as Pid);
            }

            Ok((0,))
        },
    )?;

    linker.func_wrap("get-event-data", |_: ProcessContext, _: ()| unsafe {
        let event = KERNEL.get_current_event();
        Ok((&event.data,))
    })?;

    linker.func_wrap("get-event-sender", |_: ProcessContext, _: ()| {
        let event = unsafe { KERNEL.get_current_event() };
        Ok((event.sent_by_pid as i32,))
    })?;

    linker.func_wrap(
        "add-event-handler",
        |mut ctx: ProcessContext, (name,): (WasmStr,)| {
            let ctx_ptr = &mut ctx as *mut ProcessContext;

            let name = match name.to_str(&ctx) {
                Ok(n) => n,
                Err(_) => return Ok((-1,)),
            };
            let interned_name = unsafe { KERNEL.intern_event_name(name.as_ref()) };

            let handler = unsafe {
                KERNEL
                    .get_process(ctx.data().pid)
                    .unwrap()
                    .instance
                    .get_typed_func::<(i32,), ()>(&mut *ctx_ptr, name.as_ref())
                    .unwrap()
            };

            ctx.data_mut().add_event_handler(interned_name, handler);
            Ok((0,))
        },
    )?;

    linker.func_wrap(
        "remove-event-handler",
        |mut ctx: ProcessContext, (name,): (WasmStr,)| {
            let name = match name.to_str(&ctx) {
                Ok(o) => o,
                Err(_) => return Ok((-1,)),
            };
            let interned_name = unsafe { KERNEL.intern_event_name(name.as_ref()) };

            ctx.data_mut().remove_event_handler(interned_name);
            Ok((0,))
        },
    )?;

    linker.func_wrap(
        "set-process-name",
        |ctx: ProcessContext, (name,): (WasmStr,)| {
            let name = match name.to_str(&ctx) {
                Ok(n) => n,
                Err(_) => return Ok((-1,)),
            };

            unsafe {
                if !KERNEL.set_process_name(ctx.data().pid, name.as_ref()) {
                    return Ok((-2,));
                }
            }

            Ok((0,))
        },
    )?;

    linker.func_wrap("get-process-label", |caller: ProcessContext, _: ()| {
        Ok((caller.data().label.to_owned(),))
    })?;

    linker.func_wrap(
        "get-pid-by-name",
        |ctx: ProcessContext, (name,): (WasmStr,)| {
            let name = match name.to_str(&ctx) {
                Ok(n) => n,
                Err(_) => return Ok((0,)),
            };

            let pid = unsafe { KERNEL.get_pid_by_name(name.as_ref()) as i32 };
            Ok((pid,))
        },
    )?;

    // Process

    linker.func_wrap_async("yield-now", |_: ProcessContext, _: ()| {
        Box::new(async {
            yield_now().await;
            Ok(())
        })
    })?;

    Ok(())
}
