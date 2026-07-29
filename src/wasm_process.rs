#![allow(static_mut_refs)]

use std::collections::HashSet;
use std::mem::transmute;
use std::path::PathBuf;
use std::task::Poll::{Pending, Ready};
use std::task::{Context, Waker};

use crate::KERNEL;
use crate::event::Event;
use crate::kernel::ProcessLinker;
use crate::process::Process;
use crate::ptr_cell::PtrCell;
use crate::system_functions::load_system_functions;
use tokio::task::yield_now;
use wasmtime::{Engine, Instance, Module, Store};

pub type ProcessStore = Store<Process>;

/// Returns a view of a WASM memory.
/// Note: this function exists entirely because I was having borrow errors.
#[allow(invalid_reference_casting, clippy::transmute_ptr_to_ref)]
fn get_memory_slice<'a>(instance: &'a Instance, store: &'a ProcessStore) -> &'a [u8] {
    let store_ptr = store as *const ProcessStore as *mut ProcessStore;
    unsafe {
        // TODO: replace string lookup with addding the index of "memory" to the process
        let memory = instance
            .get_memory(
                transmute::<*mut ProcessStore, &mut ProcessStore>(store_ptr),
                "memory",
            )
            .unwrap();
        memory.data(store)
    }
}

/// Returns a mutable view of a WASM memory.
/// Note: this function exists entirely because I was having borrow errors.
fn get_memory_slice_mut<'a>(instance: &'a Instance, store: &'a mut ProcessStore) -> &'a mut [u8] {
    let memory = instance.get_memory(&mut *store, "memory").unwrap();
    memory.data_mut(store)
}
/// Represents the actual running process, including its memory and functions
pub struct WasmProcess {
    pub instance: wasmtime::Instance,
    pub store: ProcessStore,
}

// Returns the names of imported drivers and libraries.
fn get_imported_modules(module: &Module) -> (Vec<&str>, Vec<&str>) {
    let mut modules = module.imports().map(|i| i.module()).collect::<HashSet<_>>();
    // Ignore as it's given my the kernel's functions, and not a driver or library.
    modules.remove("env");
    modules.into_iter().partition(|m| m.starts_with("driver_"))
}

// Dynamically loads the wasm library file
fn load_libraries(
    linker: &mut ProcessLinker,
    mut store: &mut ProcessStore,
    engine: &Engine,
    libraries: &[&str],
) -> wasmtime::Result<()> {
    for library in libraries.iter().cloned() {
        let mut library_path = PathBuf::new();
        library_path.push("lib");
        library_path.push(library);
        library_path.set_extension("wasm");

        unsafe {
            if let Ok(bytes) = KERNEL.read_file(&library_path) {
                let module = Module::new(engine, bytes)?;
                linker.module(&mut store, library, &module)?;
            }
        }
    }

    Ok(())
}

impl WasmProcess {
    pub async fn new(binary: Vec<u8>, engine: &Engine, process: Process) -> wasmtime::Result<Self> {
        // Modules are compiled from text or binary
        let module = Module::new(engine, binary)?;
        let mut linker = wasmtime::Linker::new(engine);

        let (imported_drivers, imported_libraries) = get_imported_modules(&module);

        // Load functions
        load_system_functions(&mut linker)?;
        unsafe {
            // TODO: replace with checking which drivers the compiled module references and only
            // loading the ones it needs
            KERNEL.load_driver_functions(&mut linker, &imported_drivers)?;
        }
        // TODO: look inside the `lib` directory and load libraries from `imported_libraries`
        println!("Imported drivers: {:?}", imported_drivers);
        println!("Imported libraries: {:?}", imported_libraries);

        // All wasm objects operate in the context of a store.
        // A store is used to store host-specific data of a given type.
        let mut store = Store::new(engine, process);

        load_libraries(&mut linker, &mut store, engine, &imported_libraries)?;

        // Configure preemptive interuption
        // TODO: figure out how this actually works
        store.epoch_deadline_async_yield_and_update(1);
        store.set_epoch_deadline(1);

        let instance = linker.instantiate_async(&mut store, &module).await?;

        Ok(Self { instance, store })
    }

    /// Gets a slice of memory
    pub fn get_memory(&self, address: usize, len: usize) -> &[u8] {
        let memory = get_memory_slice(&self.instance, &self.store);
        &memory[address..(address + len)]
    }

    pub fn get_memory_mut(&mut self, address: usize, len: usize) -> &mut [u8] {
        let memory = get_memory_slice_mut(&self.instance, &mut self.store);
        &mut memory[address..(address + len)]
    }

    /// Sets a slice of memory. The length of the slice is given by the lenght of the value
    pub fn set_memory(&mut self, address: usize, value: &[u8]) {
        let memory = get_memory_slice_mut(&self.instance, &mut self.store);
        let memory = &mut memory[address..];
        if memory.len() < value.len() {
            panic!("Attempted to set memory region to value larger than region allows");
        }

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

                    // TODO: move this into a system call like Linux's `wait`, so the process hangs
                    // out in memory waiting for its data to be collected.
                    unsafe {
                        KERNEL.delete_process(pid);
                    }

                    return code;
                }
                Pending => {
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
                KERNEL.set_current_event(&raw mut event);
                let length = event.data().len();

                let result = handler.call_async(&mut self.store, length as i32).await;

                if let Err(e) = result {
                    let event_name = KERNEL.get_event_name(sym);
                    eprintln!("Error in event handler {}: {}", event_name, e);
                }

                KERNEL.end_current_event();
            }
        }
    }
}
