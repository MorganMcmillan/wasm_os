#![allow(static_mut_refs)]

use std::collections::HashSet;
use std::mem::transmute;
use std::path::PathBuf;
use std::task::Poll::{Pending, Ready};
use std::task::{Context, Waker};

use crate::event::Event;
use crate::graphics::load_graphics_functions;
use crate::kernel::{Kernel, Pid, ProcessLinker};
use crate::mut_cell::MutCell;
use crate::process::Process;
use crate::ptr_cell::PtrCell;
use crate::system_functions::load_system_functions;
use tokio::task::yield_now;
use wasmtime::{Engine, Instance, Memory, Module, ModuleExport, Store};

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

/// Returns a view of a WASM memory.
/// Note: this function exists entirely because I was having borrow errors.
#[allow(invalid_reference_casting, clippy::transmute_ptr_to_ref)]
fn get_memory_slice<'a, T>(
    instance: &'a Instance,
    store: &'a ProcessStore<T>,
    mem_index: &ModuleExport,
) -> &'a [u8] {
    let store_ptr = store as *const ProcessStore<T> as *mut ProcessStore<T>;
    get_wasm_memory(instance, store_ptr, mem_index).data(store)
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
/// Represents the actual running process, including its memory and functions
pub struct WasmProcess<T: 'static> {
    pub instance: wasmtime::Instance,
    pub store: ProcessStore<T>,
}

// Returns the names of imported drivers and libraries.
fn get_imported_modules(module: &Module) -> (Vec<&str>, Vec<&str>) {
    let mut modules = module.imports().map(|i| i.module()).collect::<HashSet<_>>();
    // Ignore as it's given my the kernel's functions, and not a driver or library.
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
    for library in libraries.iter().cloned() {
        let mut library_path = PathBuf::new();
        library_path.push("lib");
        library_path.push(library);
        library_path.set_extension("wasm");

        if let Ok(bytes) = kernel.read_file(&library_path) {
            let module = Module::new(engine, bytes)?;
            linker.module(&mut store, library, &module)?;
        }
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

    /// Gets a slice of memory
    pub fn get_memory(&self, address: usize, len: usize) -> &[u8] {
        let mem_index = self.store.data().memory_export.unwrap();
        let memory = get_memory_slice(&self.instance, &self.store, &mem_index);
        &memory[address..(address + len)]
    }

    pub fn get_memory_mut(&mut self, address: usize, len: usize) -> &'static mut [u8] {
        let mem_index = self.store.data().memory_export.unwrap();
        let memory = get_memory_slice_mut(&self.instance, &mut self.store, &mem_index);
        unsafe { std::mem::transmute(&mut memory[address..(address + len)]) }
    }

    /// Sets a slice of memory. The length of the slice is given by the lenght of the value
    pub fn set_memory(&mut self, address: usize, value: &[u8]) {
        let memory = self.get_memory_mut(address, value.len());

        for (i, &byte) in value.iter().enumerate() {
            memory[i] = byte;
        }
    }

    pub async fn run(&mut self) -> i32 {
        let pid = self.store.data().pid;
        let mut self_cell = PtrCell::new(self as *mut Self);

        let run = self
            .instance
            .get_typed_func::<(), i32>(&mut self.store, "run")
            .expect("Expected the program to have an exported run function.");
        let mut main_loop = Box::pin(run.call_async(&mut self.store, ()));

        loop {
            self_cell.get_mut().process_queue().await;

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
                    self_cell.get_mut().store.set_epoch_deadline(1);
                    yield_now().await;
                }
            }
        }
    }

    async fn process_queue(&mut self) {
        let mut old_event_queue = Vec::new();
        std::mem::swap(&mut old_event_queue, &mut self.store.data_mut().event_queue);

        for event in old_event_queue {
            self.process_event(event).await;
        }
    }

    async fn process_event(&mut self, mut event: Event) {
        unsafe {
            let self_ptr = self as *mut Self;

            let sym = event.interned_name();

            if let Some(handler) = (*self_ptr).store.data().event_handlers.get(&sym) {
                self.store
                    .data()
                    .kernel
                    .borrow_static()
                    .set_current_event(&raw mut event);
                let length = event.data().len();

                let result = handler.call_async(&mut self.store, length as i32).await;

                if let Err(e) = result {
                    let event_name = self.store.data().kernel.get_event_name(sym);
                    eprintln!("Error in event handler {}: {}", event_name, e);
                }

                self.store.data().kernel.borrow_static().end_current_event();
            }
        }
    }
}
