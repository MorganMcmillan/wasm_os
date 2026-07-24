use crate::kernel::{Kernel, Pid};
use crate::process::Process;
use crate::ptr_cell::PtrCell;
use tokio::task::yield_now;
use wasmtime::*;

pub type KernelStore = Store<PtrCell<Kernel>>;
type KernelCaller<'a> = Caller<'a, PtrCell<Kernel>>;

#[allow(mismatched_lifetime_syntaxes)]
fn get_memory(
    caller: *const KernelCaller,
    pid: Pid,
    mem_ptr: i32,
    mem_len: i32,
) -> Result<&[u8], i32> {
    unsafe {
        let Some(process) = (*caller).data().get().get_process(pid) else {
            return Err(1);
        };
        let process_ptr = process as *const Process;

        let mem = (*process_ptr).get_memory(mem_ptr as usize, mem_len as usize);
        Ok(mem)
    }
}

#[allow(mismatched_lifetime_syntaxes)]
fn get_str(caller: *const KernelCaller, pid: Pid, str_ptr: i32, str_len: i32) -> Result<&str, i32> {
    let string = get_memory(caller, pid, str_ptr, str_len)?;
    let Ok(string) = str::from_utf8(string) else {
        return Err(2);
    };
    Ok(string)
}

/// Loads all core system functions into the program.
/// TODO: allow drivers to register their own functions through this or a similar method.
fn load_system_functions(linker: &mut Linker<PtrCell<Kernel>>) -> wasmtime::Result<()> {
    // Kernel methods

    linker.func_wrap(
        "env",
        "debug_print",
        |caller: KernelCaller, str_ptr: i32, str_len: i32| {
            let pid = caller.data().get().get_current_pid();
            if let Ok(string) = get_str(&caller, pid, str_ptr, str_len) {
                println!("{string}");
            }
        },
    )?;

    linker.func_wrap(
        "env",
        "send_event",
        |mut caller: KernelCaller,
         name_ptr: i32,
         name_len: i32,
         data_ptr: i32,
         data_len: i32,
         to_pid: i32|
         -> i32 {
            let pid = caller.data().get().get_current_pid();

            let name = match get_str(&caller, pid, name_ptr, name_len) {
                Ok(name) => name,
                Err(e) => return e,
            };

            let data = match get_memory(&caller, pid, data_ptr, data_len) {
                Ok(d) => d,
                Err(e) => return e,
            };

            caller
                .data_mut()
                .get_mut()
                .send_event(name, data, pid, to_pid as Pid);

            0
        },
    )?;

    linker.func_wrap(
        "env",
        "get_event_data",
        |mut caller: KernelCaller, buf_ptr: i32| unsafe {
            let caller_ptr = &mut caller as *mut KernelCaller;
            let event = caller.data().get().get_current_event();
            let pid = caller.data().get().get_current_pid();
            let process = (*caller_ptr)
                .data_mut()
                .get_mut()
                .get_process_mut(pid)
                .unwrap();
            process.set_memory(buf_ptr as usize, &event.data);
        },
    )?;

    linker.func_wrap("env", "get_event_sender", |caller: KernelCaller| -> i32 {
        let event = caller.data().get().get_current_event();
        event.sent_by_pid as i32
    })?;

    linker.func_wrap(
        "env",
        "add_event_handler",
        |mut caller: KernelCaller, name_ptr: i32, name_len: i32, handler: Func| -> i32 {
            unsafe {
                let caller_ptr = &mut caller as *mut KernelCaller;

                let name = {
                    let process = caller.data().get().get_current_process();
                    let bytes = process.get_memory(name_ptr as usize, name_len as usize);
                    let Ok(name) = str::from_utf8(bytes) else {
                        return 1;
                    };
                    name
                };
                let interned_name = (*caller_ptr).data_mut().get_mut().intern_event_name(name);

                let process = caller.data_mut().get_mut().get_current_process_mut();
                process.add_event_handler(interned_name, handler);
            }
            0
        },
    )?;

    linker.func_wrap(
        "env",
        "remove_event_handler",
        |mut caller: KernelCaller, name_ptr: i32, name_len: i32| -> i32 {
            unsafe {
                let caller_ptr = &mut caller as *mut KernelCaller;
                let pid = caller.data().get().get_current_pid();

                let name = match get_str(caller_ptr, pid, name_ptr, name_len) {
                    Ok(o) => o,
                    Err(e) => return e,
                };
                let interned_name = (*caller_ptr).data_mut().get_mut().intern_event_name(name);

                let process = caller.data_mut().get_mut().get_current_process_mut();
                process.remove_event_handler(interned_name);
            }
            0
        },
    )?;

    // Draw state
    linker.func_wrap(
        "env",
        "set_active_framebuffer",
        |mut caller: KernelCaller, framebuffer: i32| {
            let pid = caller.data().get().get_current_pid();
            caller
                .data_mut()
                .get_mut()
                .drawstate
                .set_framebuffer_address(pid, framebuffer as u32);
            println!("Called set_active_framebuffer.");
        },
    )?;

    // Input
    linker.func_wrap("env", "get_mouse_x", |caller: KernelCaller| {
        caller.data().get().mousestate.x as i32
    })?;

    linker.func_wrap("env", "get_mouse_y", |caller: KernelCaller| {
        caller.data().get().mousestate.y as i32
    })?;

    // Process

    linker.func_wrap_async("env", "yield_now", |_: KernelCaller, _: ()| {
        Box::new(async {
            yield_now().await;
        })
    })?;

    Ok(())
}

pub struct WasmState {
    pub instance: Instance,
    pub store: KernelStore,
}

impl WasmState {
    pub async fn new(
        binary: Vec<u8>,
        engine: &Engine,
        kernel: *mut Kernel,
    ) -> wasmtime::Result<Self> {
        // Modules are compiled from text or binary
        let module = Module::new(engine, binary)?;

        // Linkers expose host functions
        let mut linker = Linker::new(engine);

        if let Err(e) = load_system_functions(&mut linker) {
            println!("Error loading system functions: {:?}", e);
            return Err(e);
        };

        // All wasm objects operate in the context of a store.
        // A store is used to store host-specific data of a given type.
        let mut store = Store::new(engine, PtrCell::new(kernel));
        let instance = match linker.instantiate_async(&mut store, &module).await {
            Err(e) => {
                println!("Error creating instance: {:?}", e);
                return Err(e);
            }
            Ok(i) => i,
        };

        Ok(Self { instance, store })
    }

    pub fn kernel(&self) -> &Kernel {
        self.store.data().get()
    }

    pub fn kernel_mut(&mut self) -> &mut Kernel {
        self.store.data_mut().get_mut()
    }

    // pub fn kernel_ptr(&self) -> *mut Kernel {
    //     self.store.data().inner
    // }
}
