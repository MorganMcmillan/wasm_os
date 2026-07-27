#![allow(static_mut_refs)]

use std::task::Poll::{Pending, Ready};
use std::task::{Context, Waker};

use crate::KERNEL;
use crate::event::Event;
use crate::process::Process;
use crate::ptr_cell::PtrCell;
use crate::system_functions::load_system_functions;
use tokio::task::yield_now;
use wasmtime::{Engine, Module, Store};

pub type ProcessStore = Store<Process>;

/// Represents the actual running process, including its memory and functions
pub struct WasmProcess {
    pub instance: wasmtime::Instance,
    pub store: ProcessStore,
}

impl WasmProcess {
    pub async fn new(binary: Vec<u8>, engine: &Engine, process: Process) -> wasmtime::Result<Self> {
        // Modules are compiled from text or binary
        let module = Module::new(engine, binary)?;
        let mut linker = wasmtime::Linker::new(engine);

        // Load functions
        load_system_functions(&mut linker)?;
        unsafe {
            KERNEL.load_driver_functions(&mut linker)?;
        }

        // All wasm objects operate in the context of a store.
        // A store is used to store host-specific data of a given type.
        let mut store = Store::new(engine, process);

        // Configure preemptive interuption
        store.epoch_deadline_async_yield_and_update(1);
        store.set_epoch_deadline(1);

        let instance = linker.instantiate_async(&mut store, &module).await?;

        Ok(Self { instance, store })
    }

    pub async fn run(&mut self) -> i32 {
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
            let self_ptr = self as *mut Self;

            let sym = event.interned_name;

            if let Some(handler) = (*self_ptr).store.data().event_handlers.get(&sym) {
                KERNEL.set_current_event(&raw mut event);
                let length = event.data.len();

                let result = self
                    .store
                    .run_concurrent(async |accessor| -> wasmtime::Result<_> {
                        handler.call_concurrent(accessor, (length as u32,)).await?;
                        Ok(())
                    })
                    .await;

                if let Err(e) = result {
                    let event_name = KERNEL.get_event_name(sym);
                    eprintln!("Error in event handler {}: {}", event_name, e);
                } else if let Ok(Err(e)) = result {
                    let event_name = KERNEL.get_event_name(sym);
                    eprintln!("Error in event handler {}: {}", event_name, e);
                }
            }
        }
    }
}
