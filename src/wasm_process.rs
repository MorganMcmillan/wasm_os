#![allow(static_mut_refs)]

use std::task::Poll::{Pending, Ready};
use std::task::{Context, Waker};

use crate::KERNEL;
use crate::event::Event;
use crate::process::Process;
use crate::ptr_cell::PtrCell;
use crate::system_functions::load_system_functions;
use tokio::task::yield_now;
use wasmtime::component::Component;
use wasmtime::{Engine, Store};

pub type ProcessStore = Store<Process>;

/// Represents the actual running process, including its memory and functions
pub struct WasmProcess {
    pub instance: wasmtime::component::Instance,
    pub store: ProcessStore,
}

impl WasmProcess {
    pub async fn new(binary: Vec<u8>, engine: &Engine, process: Process) -> wasmtime::Result<Self> {
        // Modules are compiled from text or binary
        let component = Component::new(engine, binary)?;

        // Load system functions

        let mut linker = wasmtime::component::Linker::new(engine);

        let mut root = linker.root();

        if let Err(e) = load_system_functions(&mut root) {
            eprintln!("Error loading system functions: {:?}", e);
            return Err(e);
        };

        unsafe {
            KERNEL.load_driver_functions(&mut root)?;
        }

        wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;

        // All wasm objects operate in the context of a store.
        // A store is used to store host-specific data of a given type.
        let mut store = Store::new(engine, process);
        let instance = match linker.instantiate_async(&mut store, &component).await {
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
            .get_typed_func::<(), (i32,)>(&mut self.store, "run")
            .unwrap();
        let mut main_loop = Box::pin(run.call_async(&mut self.store, ()));

        loop {
            self_cell.get_mut().process_queue().await;

            let poll_result =
                Future::poll(main_loop.as_mut(), &mut Context::from_waker(Waker::noop()));
            match poll_result {
                Ready(result) => match result {
                    Ok((code,)) => {
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
}
