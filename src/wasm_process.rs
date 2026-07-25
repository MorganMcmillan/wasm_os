#![allow(static_mut_refs)]

use std::task::Poll::{Pending, Ready};
use std::task::{Context, Waker};

use crate::KERNEL;
use crate::event::Event;
use crate::process::Process;
use crate::ptr_cell::PtrCell;
use crate::system_functions::load_system_functions;
use tokio::task::yield_now;
use wasmtime::*;

pub type ProcessStore = Store<Process>;

/// Returns a view of a WASM memory.
/// Note: this function exists entirely because I was having borrow errors.
#[allow(invalid_reference_casting)]
fn get_memory_slice<'a>(instance: &'a Instance, store: &'a ProcessStore) -> &'a [u8] {
    let store_ptr = store as *const ProcessStore as *mut ProcessStore;
    unsafe {
        let memory = instance.get_memory(&mut *store_ptr, "memory").unwrap();
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
    pub instance: Instance,
    pub store: ProcessStore,
}

impl WasmProcess {
    pub async fn new(binary: Vec<u8>, engine: &Engine, process: Process) -> wasmtime::Result<Self> {
        // Modules are compiled from text or binary
        let module = Module::new(engine, binary)?;

        // Linkers expose host functions
        let mut linker = Linker::new(engine);

        if let Err(e) = load_system_functions(&mut linker) {
            eprintln!("Error loading system functions: {:?}", e);
            return Err(e);
        };

        unsafe {
            KERNEL.load_driver_functions(&mut linker)?;
        }

        // All wasm objects operate in the context of a store.
        // A store is used to store host-specific data of a given type.
        let mut store = Store::new(engine, process);
        let instance = match linker.instantiate_async(&mut store, &module).await {
            Err(e) => {
                eprintln!("Error creating instance: {:?}", e);
                return Err(e);
            }
            Ok(i) => i,
        };

        Ok(Self { instance, store })
    }

    pub async fn run(&mut self) -> i32 {
        let mut self_cell = PtrCell::new(self as *mut Self);

        let run = self
            .instance
            .get_typed_func::<(), i32>(&mut self.store, "run")
            .unwrap();
        let mut main_loop = Box::pin(run.call_async(&mut self.store, ()));

        loop {
            unsafe {
                KERNEL.set_current_pid(self_cell.get().store.data().pid);
            }

            self_cell.get_mut().process_queue().await;

            let poll_result =
                Future::poll(main_loop.as_mut(), &mut Context::from_waker(Waker::noop()));
            match poll_result {
                Ready(result) => match result {
                    Ok(code) => {
                        self_cell.get_mut().store.data_mut().exit_code = Some(code as u16);
                        return code;
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        self_cell.get_mut().store.data_mut().exit_code = Some(100);
                        return 100;
                    }
                },
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
            KERNEL.set_current_event(&raw mut event);
            let self_ptr = self as *mut Self;

            let sym = event.interned_name;
            if let Some(handler) = (*self_ptr).store.data().event_handlers.get(&sym) {
                let result = handler
                    .call_async(&mut self.store, (event.length as i32,))
                    .await;
                if let Err(e) = result {
                    let event_name = KERNEL.get_event_name(sym);
                    eprintln!("Error in event handler {}: {}", event_name, e);
                }
            }
        }
    }
    //
    // Memory

    /// Gets a slice of memory
    pub fn get_memory(&self, address: usize, len: usize) -> &[u8] {
        let memory = get_memory_slice(&self.instance, &self.store);
        &memory[address..(address + len)]
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
}
