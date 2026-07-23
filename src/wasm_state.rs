use crate::draw;
use crate::kernel::Kernel;
use crate::ptr_cell::PtrCell;
use wasmtime::*;

type KernelStore = Store<PtrCell<Kernel>>;

/// Returns a view of a WASM memory.
/// Note: this function exists entirely because I was having borrow errors.
fn get_memory_slice<'a>(instance: &'a Instance, store: &'a mut KernelStore) -> &'a [u8] {
    let memory = instance.get_memory(&mut *store, "memory").unwrap();
    memory.data(store)
}

/// Gets the region of memory associated with the active framebuffer's program.
/// The returned slice is exactly 384*216 bytes.
fn get_framebuffer<'a>(
    instance: &'a Instance,
    store: &'a mut KernelStore,
    address: usize,
) -> &'a [u8] {
    let memory = get_memory_slice(instance, store);
    &memory[address..(address + draw::FRAMEBUFFER_SIZE)]
}

/// Loads all core system functions into the program.
/// TODO: allow drivers to register their own functions through this or a similar method.
fn load_system_functions(linker: &mut Linker<PtrCell<Kernel>>) -> wasmtime::Result<()> {
    // Load "set_active_framebuffer"
    linker.func_wrap(
        "env",
        "set_active_framebuffer",
        |mut caller: Caller<PtrCell<Kernel>>, framebuffer: i32| {
            caller.data_mut().get_mut().drawstate.framebuffer_address = Some(framebuffer as u32);
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

    /// Calls the `init` function of the program
    pub fn init(&mut self) -> wasmtime::Result<()> {
        let init_func = self
            .instance
            .get_typed_func::<(), ()>(&mut self.store, "init")?;
        init_func.call(&mut self.store, ())?;
        Ok(())
    }

    /// Calls the `update` function of the program
    pub fn update(&mut self) {
        let update_func = self
            .instance
            .get_typed_func::<(), ()>(&mut self.store, "update")
            .unwrap();
        update_func.call(&mut self.store, ()).unwrap();
    }

    pub fn kernel(&self) -> &Kernel {
        self.store.data().get()
    }

    pub fn kernel_mut(&mut self) -> &mut Kernel {
        self.store.data_mut().get_mut()
    }

    pub fn kernel_ptr(&self) -> *mut Kernel {
        self.store.data().inner
    }

    /// Draws the program's framebuffer to a texture
    pub fn upload_framebuffer(&mut self) {
        unsafe {
            let kernel: *mut Kernel = self.store.data_mut().inner;
            if let Some(address) = (*kernel).drawstate.framebuffer_address {
                let framebuffer =
                    get_framebuffer(&self.instance, &mut self.store, address as usize);
                (*kernel).drawstate.upload_framebuffer(framebuffer);
            }
        }
    }
}
