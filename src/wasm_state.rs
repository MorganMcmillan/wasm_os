use crate::event::Event;
use crate::kernel::{Kernel, Pid};
use crate::process::Process;
use crate::ptr_cell::PtrCell;
use wasmtime::*;

pub type KernelStore = Store<PtrCell<Kernel>>;
type KernelCaller<'a> = Caller<'a, PtrCell<Kernel>>;

/// Loads all core system functions into the program.
/// TODO: allow drivers to register their own functions through this or a similar method.
fn load_system_functions(linker: &mut Linker<PtrCell<Kernel>>) -> wasmtime::Result<()> {
    // Kernel methods
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

            let (name, data) = unsafe {
                let Some(process) = caller.data().get().get_process(pid) else {
                    return 1;
                };
                let process_ptr = process as *const Process;

                let name = (*process_ptr).get_memory(name_ptr as usize, name_len as usize);
                let Ok(name) = str::from_utf8(name) else {
                    return 2;
                };

                let data = (*process_ptr).get_memory(data_ptr as usize, data_len as usize);
                (name, data)
            };

            caller
                .data_mut()
                .get_mut()
                .send_event(name, data, pid, to_pid as Pid);

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
        },
    )?;

    // Input
    linker.func_wrap("env", "get_mouse_x", |caller: KernelCaller| {
        caller.data().get().mousestate.x as i32
    })?;

    linker.func_wrap("env", "get_mouse_y", |caller: KernelCaller| {
        caller.data().get().mousestate.y as i32
    })?;

    Ok(())
}

pub struct WasmState {
    pub instance: Instance,
    pub store: KernelStore,
}

impl WasmState {
    pub fn new(binary: Vec<u8>, engine: &Engine, kernel: *mut Kernel) -> wasmtime::Result<Self> {
        // Modules are compiled from text or binary
        let module = Module::new(engine, binary)?;

        // Linkers expose host functions
        let mut linker = Linker::new(engine);

        load_system_functions(&mut linker)?;

        // All wasm objects operate in the context of a store.
        // A store is used to store host-specific data of a given type.
        let mut store = Store::new(engine, PtrCell::new(kernel));

        Ok(Self {
            instance: linker.instantiate(&mut store, &module)?,
            store,
        })
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
