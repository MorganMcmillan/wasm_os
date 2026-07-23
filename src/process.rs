use std::collections::HashMap;
use std::task::Poll::{Pending, Ready};
use std::task::{Context, Waker};
use tokio::task::JoinHandle;

use string_interner::symbol::SymbolU32;
use wasmtime::{Instance, TypedFunc};

use crate::draw;
use crate::event::Event;
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

type HandlerFn = TypedFunc<(), ()>;

pub struct Process {
    event_queue: Vec<Event>,
    event_handlers: HashMap<SymbolU32, HandlerFn>,
    wasm_state: WasmState,
    join_handle: Option<JoinHandle<i32>>,
}

impl Process {
    pub fn new(wasm_state: WasmState) -> Self {
        Self {
            event_queue: Vec::new(),
            event_handlers: HashMap::new(),
            wasm_state,
            join_handle: None,
        }
    }

    pub fn set_join_handle(&mut self, join_handle: JoinHandle<i32>) {
        self.join_handle = Some(join_handle);
    }

    pub fn push_event(&mut self, event: Event) {
        self.event_queue.push(event);
    }

    pub async fn run(&mut self) -> i32 {
        let self_ptr = self as *mut Self;

        let run = self
            .wasm_state
            .instance
            .get_typed_func::<(), i32>(&mut self.wasm_state.store, "run")
            .unwrap();
        let mut main_loop = Box::pin(run.call_async(&mut self.wasm_state.store, ()));

        loop {
            unsafe {
                (*self_ptr).process_queue();
            }

            let mut context = Context::from_waker(Waker::noop());
            match Future::poll(main_loop.as_mut(), &mut context) {
                Ready(result) => match result {
                    Ok(code) => return code,
                    Err(e) => {
                        eprintln!("{e}");
                        return 100;
                    }
                },
                Pending => {}
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

    fn process_event(&mut self, event: Event) {
        self.wasm_state
            .kernel_mut()
            .set_current_event(&raw const event);

        let sym = event.interned_name;
        if let Some(handler) = self.event_handlers.get(&sym) {
            let result = handler.call(&mut self.wasm_state.store, ());
            if let Err(e) = result {
                let event_name = self.wasm_state.kernel_mut().get_event_name(sym);
                eprintln!("Error in event handler {}: {}", event_name, e);
            }
        }
    }

    /// Gets a slice of memory
    pub fn get_memory(&self, address: usize, len: usize) -> &[u8] {
        let memory = get_memory_slice(&self.wasm_state.instance, &self.wasm_state.store);
        &memory[address..(address + len)]
    }

    /// Gets the region of memory associated with the active framebuffer's program.
    /// The returned slice is exactly 384*216 bytes.
    pub fn get_framebuffer(&self, address: usize) -> &[u8] {
        self.get_memory(address, draw::FRAMEBUFFER_SIZE)
    }
}
