#![allow(static_mut_refs)]

use std::{
    collections::HashSet,
    mem::transmute,
    path::PathBuf,
    task::{
        Context,
        Poll::{Pending, Ready},
        Waker,
    },
};

use crate::{
    cell::mut_cell::MutCell,
    cell::ptr_cell::PtrCell,
    event::Event,
    graphics::load_graphics_functions,
    kernel::{Kernel, Pid, ProcessLinker},
    process::Process,
    system_functions::load_system_functions,
};

use tokio::task::yield_now;
use wasmtime::{Engine, Instance, Memory, Module, ModuleExport, Store, TypedFunc};

pub type ProcessStore<T> = Store<Process<T>>;

#[allow(invalid_reference_casting, clippy::transmute_ptr_to_ref)]
fn get_wasm_memory<T>(
    instance: &Instance,
    store_ptr: *mut ProcessStore<T>,
    mem_index: &ModuleExport,
) -> Memory {
    unsafe {
        instance
            .get_module_export(
                transmute::<*mut ProcessStore<T>, &mut ProcessStore<T>>(store_ptr),
                mem_index,
            )
            .unwrap()
            .into_memory()
            .unwrap()
    }
}

/// Returns a mutable view of a WASM memory.
/// Note: this function exists entirely because I was having borrow errors.
fn get_memory_slice_mut<'a, T>(
    instance: &'a Instance,
    store: &'a mut ProcessStore<T>,
    mem_index: &ModuleExport,
) -> &'a mut [u8] {
    get_wasm_memory(instance, store, mem_index).data_mut(store)
}
/// A wasm process represents the actual running process, including its memory and functions.
/// It holds the process' data in `store`.
pub struct WasmProcess<T: 'static> {
    pub instance: wasmtime::Instance,
    pub store: ProcessStore<T>,
}

// Returns the names of imported drivers and libraries.
fn get_imported_modules(module: &Module) -> (Vec<&str>, Vec<&str>) {
    let mut modules = module.imports().map(|i| i.module()).collect::<HashSet<_>>();
    // Ignore as it's given by the kernel's functions, and not a driver or library.
    modules.remove("env");
    modules.into_iter().partition(|m| m.starts_with("driver_"))
}

// Dynamically loads the wasm library file
fn load_libraries<T>(
    kernel: &'static MutCell<Kernel<T>>,
    linker: &mut ProcessLinker<T>,
    mut store: &mut ProcessStore<T>,
    engine: &Engine,
    libraries: &[&str],
) -> wasmtime::Result<()> {
    fn load_library_from<T>(
        kernel: &'static MutCell<Kernel<T>>,
        path: &str,
        library: &str,
    ) -> Option<Vec<u8>> {
        let mut library_path = PathBuf::new();
        library_path.push(path);
        library_path.push(library);
        library_path.set_extension("wasm");

        kernel.read_file(library_path).ok()
    }

    for library in libraries.iter().cloned() {
        let Some(bytes) = load_library_from(kernel, "rom/lib", library)
            .or_else(|| load_library_from(kernel, "bios/lib", library))
            .or_else(|| load_library_from(kernel, "lib", library))
        else {
            continue;
        };

        let module = Module::new(engine, bytes)?;

        linker.module(&mut store, library, &module)?;
    }

    Ok(())
}

impl<T> WasmProcess<T> {
    pub async fn new(
        kernel: &'static MutCell<Kernel<T>>,
        binary: Vec<u8>,
        engine: &Engine,
        mut process: Process<T>,
    ) -> wasmtime::Result<Self> {
        // Modules are compiled from text or binary
        let module = Module::new(engine, binary)?;
        process.memory_export = module.get_export_index("memory");

        let mut linker = wasmtime::Linker::new(engine);

        let (imported_drivers, imported_libraries) = get_imported_modules(&module);

        // Load functions
        load_system_functions(&mut linker)?;
        load_graphics_functions(&mut linker)?;
        kernel
            .borrow_static()
            .load_driver_functions(&mut linker, &imported_drivers)?;
        println!("Imported drivers: {:?}", imported_drivers);
        println!("Imported libraries: {:?}", imported_libraries);

        // All wasm objects operate in the context of a store.
        // A store is used to store host-specific data of a given type.
        let mut store = Store::new(engine, process);

        load_libraries(kernel, &mut linker, &mut store, engine, &imported_libraries)?;

        // Configure preemptive interuption
        store.epoch_deadline_async_yield_and_update(1);
        store.set_epoch_deadline(1);

        let instance = linker.instantiate_async(&mut store, &module).await?;

        Ok(Self { instance, store })
    }

    pub fn get_entire_memory(&mut self) -> &'static mut [u8] {
        let mem_index = self.store.data().memory_export.unwrap();
        let memory = get_memory_slice_mut(&self.instance, &mut self.store, &mem_index);
        // SAFETY: this method must only be called in system functions, where the lifetime of this
        // process is valid.
        unsafe { std::mem::transmute(memory) }
    }

    /// Gets a mutable slice of memory from this process.
    pub fn get_memory(&mut self, address: usize, len: usize) -> &'static mut [u8] {
        &mut self.get_entire_memory()[address..(address + len)]
    }

    /// Sets a slice of memory. The length of the slice is given by the lenght of the value
    pub fn set_memory(&mut self, address: usize, value: &[u8]) {
        let memory = self.get_memory(address, value.len());

        for (i, &byte) in value.iter().enumerate() {
            memory[i] = byte;
        }
    }

    /// Runs this process.
    /// This method does not wait for the process to finish, rather it creates a new task and eventually returns an exit code.
    /// This calls the exported wasm function `run`, which is expected to have the signature `() ->
    /// i32`
    pub async fn run(&mut self) -> i32 {
        let pid = self.store.data().pid;
        let mut self_cell = PtrCell::new(self as *mut Self);

        let run = self
            .instance
            .get_typed_func::<(), i32>(&mut self.store, "run")
            .expect("Expected the program to have an exported run function.");
        let mut main_loop = Box::pin(run.call_async(&mut self.store, ()));

        loop {
            self_cell.get_mut().store.set_epoch_deadline(1);
            self_cell.get_mut().process_queue().await;
            self_cell.get_mut().store.set_epoch_deadline(1);

            let poll_result =
                Future::poll(main_loop.as_mut(), &mut Context::from_waker(Waker::noop()));
            match poll_result {
                Ready(result) => {
                    let code = match result {
                        Ok(code) => {
                            self_cell.get_mut().store.data_mut().exit_code = Some(code as u16);
                            code
                        }
                        Err(e) => {
                            eprintln!("{e}");
                            self_cell.get_mut().store.data_mut().exit_code = Some(100);
                            100
                        }
                    };

                    // Notify parent that child has exited.
                    let kernel = self_cell.get().store.data().kernel;
                    let parent = self_cell.get().store.data().parent_pid;
                    if parent != Pid::default() {
                        let pid = self_cell.get().store.data().pid;
                        kernel.borrow_static().send_event(
                            "child_exited",
                            &code.to_le_bytes(),
                            pid,
                            parent,
                        );
                    }

                    kernel.borrow_static().delete_process(pid);

                    return code;
                }
                Pending => {
                    yield_now().await;
                }
            }
        }
    }

    /// Process all events in the event queue.
    async fn process_queue(&mut self) {
        // Prevent prepared data from being overwriten by a handler, in the rare case that the
        // process interupts when data is being prepared.
        let previous_data = self.store.data_mut().data.take();

        let mut old_event_queue = Vec::new();
        std::mem::swap(&mut old_event_queue, &mut self.store.data_mut().event_queue);

        for event in old_event_queue {
            self.process_event(event).await;
        }

        self.store.data_mut().data = previous_data;
    }

    /// Processes a single event.
    async fn process_event(&mut self, event: Event) {
        async fn handle_event<T>(
            mut store: &mut ProcessStore<T>,
            mut event: Event,
            handler: &TypedFunc<i32, ()>,
        ) {
            let sym = event.interned_name();

            store
                .data()
                .kernel
                .borrow_static()
                .set_current_event(&raw mut event);
            let length = event.data().len();

            let result = handler.call_async(&mut store, length as i32).await;

            if let Err(e) = result {
                let event_name = store.data().kernel.get_event_name(sym);
                eprintln!("Error in event handler {}: {}", event_name, e);
            }

            store.data().kernel.borrow_static().end_current_event();
        }

        unsafe {
            let self_ptr = self as *mut Self;
            let sym = event.interned_name();

            if let Some(handler) = (*self_ptr).store.data().event_handlers.get(&sym) {
                handle_event(&mut self.store, event, handler).await;
            } else if let Some(default_handler) = &(*self_ptr).store.data().default_event_handler {
                handle_event(&mut self.store, event, default_handler).await;
            }
        }
    }

    pub fn get_exported_function(&mut self, handler_index: i32) -> Option<wasmtime::Func> {
        // TODO: store the table index inside the process itself
        self.instance
            .get_table(&mut self.store, "__indirect_function_table")
            .or_else(|| self.instance.get_table(&mut self.store, "table"))
            .and_then(|table| table.get(&mut self.store, handler_index as u64))
            .and_then(|val| val.as_func().flatten().copied())
    }
}
