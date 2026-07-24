use std::collections::HashMap;
use std::task::Poll::{Pending, Ready};
use std::task::{Context, Waker};
use tokio::task::{JoinHandle, yield_now};

use string_interner::symbol::SymbolU32;
use wasmtime::{Func, Instance};

use crate::event::Event;
use crate::kernel::Pid;
use crate::ptr_cell::PtrCell;
use crate::wasm_state::{KernelStore, WasmState};

/// Returns a view of a WASM memory.
/// Note: this function exists entirely because I was having borrow errors.
#[allow(invalid_reference_casting)]
fn get_memory_slice<'a>(instance: &'a Instance, store: &'a KernelStore) -> &'a [u8] {
    let store_ptr = store as *const KernelStore as *mut KernelStore;
    unsafe {
        let memory = instance.get_memory(&mut *store_ptr, "memory").unwrap();
        memory.data(store)
    }
}

/// Returns a mutable view of a WASM memory.
/// Note: this function exists entirely because I was having borrow errors.
fn get_memory_slice_mut<'a>(instance: &'a Instance, store: &'a mut KernelStore) -> &'a mut [u8] {
    let memory = instance.get_memory(&mut *store, "memory").unwrap();
    memory.data_mut(store)
}

pub struct Process {
    pid: Pid,
    event_queue: Vec<Event>,
    event_handlers: HashMap<SymbolU32, Func>,
    pub wasm_state: WasmState,
    join_handle: Option<JoinHandle<i32>>,
}

impl Process {
    pub fn new(wasm_state: WasmState, pid: Pid) -> Self {
        Self {
            pid,
            event_queue: Vec::new(),
            event_handlers: HashMap::new(),
            wasm_state,
            join_handle: None,
        }
    }

    pub fn set_join_handle(&mut self, join_handle: JoinHandle<i32>) {
        self.join_handle = Some(join_handle);
    }

    // Events

    pub fn push_event(&mut self, event: Event) {
        self.event_queue.push(event);
    }

    pub fn add_event_handler(&mut self, name: SymbolU32, handler: Func) {
        self.event_handlers.insert(name, handler);
    }

    pub fn remove_event_handler(&mut self, name: SymbolU32) {
        self.event_handlers.remove(&name);
    }

    // Main Loop

    pub async fn run(&mut self) -> i32 {
        let mut self_cell = PtrCell::new(self as *mut Self);

        let run = self
            .wasm_state
            .instance
            .get_typed_func::<(), i32>(&mut self.wasm_state.store, "run")
            .unwrap();
        let mut main_loop = Box::pin(run.call_async(&mut self.wasm_state.store, ()));

        loop {
            self_cell
                .get_mut()
                .wasm_state
                .store
                .data_mut()
                .get_mut()
                .set_current_pid(self.pid);

            self_cell.get_mut().process_queue();

            let poll_result =
                Future::poll(main_loop.as_mut(), &mut Context::from_waker(Waker::noop()));
            match poll_result {
                Ready(result) => match result {
                    Ok(code) => return code,
                    Err(e) => {
                        eprintln!("{e}");
                        return 100;
                    }
                },
                Pending => {
                    yield_now().await;
                }
            }
        }
    }

    fn process_queue(&mut self) {
        let mut old_event_queue = Vec::new();
        std::mem::swap(&mut old_event_queue, &mut self.event_queue);

        for event in old_event_queue {
            self.process_event(event);
        }
    }

    fn process_event(&mut self, mut event: Event) {
        self.wasm_state
            .kernel_mut()
            .set_current_event(&raw mut event);

        let sym = event.interned_name;
        if let Some(handler) = self.event_handlers.get(&sym) {
            let result = handler.call(&mut self.wasm_state.store, &[], &mut []);
            if let Err(e) = result {
                let event_name = self.wasm_state.kernel_mut().get_event_name(sym);
                eprintln!("Error in event handler {}: {}", event_name, e);
            }
        }
    }

    // Memory

    /// Gets a slice of memory
    pub fn get_memory(&self, address: usize, len: usize) -> &[u8] {
        let memory = get_memory_slice(&self.wasm_state.instance, &self.wasm_state.store);
        &memory[address..(address + len)]
    }

    /// Sets a slice of memory. The length of the slice is given by the lenght of the value
    pub fn set_memory(&mut self, address: usize, value: &[u8]) {
        let memory = get_memory_slice_mut(&self.wasm_state.instance, &mut self.wasm_state.store);
        let memory = &mut memory[address..];
        if memory.len() < value.len() {
            panic!("Attempted to set memory region to value larger than region allows");
        }

        for (i, &byte) in value.iter().enumerate() {
            memory[i] = byte;
        }
    }
}
