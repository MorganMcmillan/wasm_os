use crate::kernel::Kernel;
use crate::ptr_cell::PtrCell;
use wasmtime::*;

pub type KernelStore = Store<PtrCell<Kernel>>;

/// Loads all core system functions into the program.
/// TODO: allow drivers to register their own functions through this or a similar method.
fn load_system_functions(linker: &mut Linker<PtrCell<Kernel>>) -> wasmtime::Result<()> {
    // Load "set_active_framebuffer"
    linker.func_wrap(
        "env",
        "set_active_framebuffer",
        |mut caller: Caller<PtrCell<Kernel>>, framebuffer: i32| {
            let pid = caller.data().get().get_current_pid();
            caller
                .data_mut()
                .get_mut()
                .drawstate
                .set_framebuffer_address(pid, framebuffer as u32);
        },
    )?;

    linker.func_wrap("env", "get_mouse_x", |caller: Caller<PtrCell<Kernel>>| {
        caller.data().get().mousestate.x as i32
    })?;

    linker.func_wrap("env", "get_mouse_y", |caller: Caller<PtrCell<Kernel>>| {
        caller.data().get().mousestate.y as i32
    })?;

    Ok(())
}

pub struct WasmState {
    pub instance: Instance,
    pub store: KernelStore,
}

impl WasmState {
    pub fn new(engine: &Engine, kernel: *mut Kernel) -> wasmtime::Result<Self> {
        let binary = std::fs::read("test.wasm").unwrap();
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
