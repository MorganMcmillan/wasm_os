use string_interner::Symbol as _;
use string_interner::symbol::SymbolU32;

use crate::kernel::{Pid, ProcessContext, ProcessLinker};
use crate::system_functions::get_memory;
use crate::system_functions::get_str;

const EVENT_HANDLER_NOT_FOUND: &str = "Could not get event handler.\nIt's possible your program was not compiled with wasm-ld having the `--export-table` flag.";

pub fn load_system_functions<T>(linker: &mut ProcessLinker<T>) -> wasmtime::Result<()> {
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
                .typed::<i32, ()>(&ctx)
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

    Ok(())
}
