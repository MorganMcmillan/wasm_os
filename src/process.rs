use wasmtime::Instance;

use crate::draw;
use crate::event::Event;
use crate::wasm_state::{KernelStore, WasmState};

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

pub struct Process {
    event_queue: Vec<Event>,
    wasm_state: WasmState,
}

impl Process {
    fn new(wasm_state: WasmState) -> Self {
        Self {
            event_queue: Vec::new(),
            wasm_state,
        }
    }

    pub fn push_event(&mut self, event: Event) {
        self.event_queue.push(event);
    }

    pub fn get_framebuffer(&mut self, address: usize) -> &[u8] {
        get_framebuffer(
            &mut self.wasm_state.instance,
            &mut self.wasm_state.store,
            address,
        )
    }
}
