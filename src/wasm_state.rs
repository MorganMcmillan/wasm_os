#![allow(static_mut_refs)]

use std::task::Poll::{Pending, Ready};
use std::task::{Context, Waker};

use crate::KERNEL;
use crate::event::Event;
use crate::kernel::Pid;
use crate::process::Process;
use crate::ptr_cell::PtrCell;
use tokio::task::yield_now;
use wasmtime::*;

pub type ProcessStore = Store<Process>;
type ProcessCaller<'a> = Caller<'a, Process>;

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

#[allow(mismatched_lifetime_syntaxes)]
fn get_memory(caller: *const ProcessCaller, mem_ptr: i32, mem_len: i32) -> Result<&[u8], i32> {
    unsafe {
        let process = KERNEL.get_process_mut((*caller).data().pid()).unwrap();

        let mem = process.get_memory(mem_ptr as usize, mem_len as usize);
        Ok(mem)
    }
}

#[allow(mismatched_lifetime_syntaxes)]
fn get_str(caller: *const ProcessCaller, str_ptr: i32, str_len: i32) -> Result<&str, i32> {
    let string = get_memory(caller, str_ptr, str_len)?;
    let Ok(string) = str::from_utf8(string) else {
        return Err(2);
    };
    Ok(string)
}

/// Loads all core system functions into the program.
/// TODO: allow drivers to register their own functions through this or a similar method.
fn load_system_functions(linker: &mut Linker<Process>) -> wasmtime::Result<()> {
    // Kernel methods

    linker.func_wrap(
        "env",
        "debug_print",
        |caller: ProcessCaller, str_ptr: i32, str_len: i32| {
            if let Ok(string) = get_str(&raw const caller, str_ptr, str_len) {
                println!("{string}");
            }
        },
    )?;

    linker.func_wrap(
        "env",
        "send_event",
        |caller: ProcessCaller,
         name_ptr: i32,
         name_len: i32,
         data_ptr: i32,
         data_len: i32,
         to_pid: i32|
         -> i32 {
            let name = match get_str(&caller, name_ptr, name_len) {
                Ok(name) => name,
                Err(e) => return e,
            };

            let data = match get_memory(&caller, data_ptr, data_len) {
                Ok(d) => d,
                Err(e) => return e,
            };

            unsafe {
                KERNEL.send_event(name, data, caller.data().pid(), to_pid as Pid);
            }

            0
        },
    )?;

    linker.func_wrap(
        "env",
        "get_event_data",
        |caller: ProcessCaller, buf_ptr: i32| unsafe {
            let event = KERNEL.get_current_event();
            // WARNING: may cause an issue
            let process = KERNEL.get_process_mut(caller.data().pid()).unwrap();
            process.set_memory(buf_ptr as usize, &event.data);
        },
    )?;

    linker.func_wrap("env", "get_event_sender", |_: ProcessCaller| -> i32 {
        let event = unsafe { KERNEL.get_current_event() };
        event.sent_by_pid as i32
    })?;

    linker.func_wrap(
        "env",
        "add_event_handler",
        |mut caller: ProcessCaller, name_ptr: i32, name_len: i32, handler: Func| -> i32 {
            let name = match get_str(&raw const caller, name_ptr, name_len) {
                Ok(n) => n,
                Err(e) => return e,
            };
            let interned_name = unsafe { KERNEL.intern_event_name(name) };

            caller.data_mut().add_event_handler(interned_name, handler);
            0
        },
    )?;

    linker.func_wrap(
        "env",
        "remove_event_handler",
        |mut caller: ProcessCaller, name_ptr: i32, name_len: i32| -> i32 {
            let name = match get_str(&raw const caller, name_ptr, name_len) {
                Ok(o) => o,
                Err(e) => return e,
            };
            let interned_name = unsafe { KERNEL.intern_event_name(name) };

            caller.data_mut().remove_event_handler(interned_name);
            0
        },
    )?;

    // Draw state
    linker.func_wrap(
        "env",
        "set_active_framebuffer",
        |caller: ProcessCaller, framebuffer: i32| {
            let pid = caller.data().pid();
            unsafe {
                KERNEL
                    .drawstate
                    .set_framebuffer_address(pid, framebuffer as u32);
                KERNEL.test_data = 6767;
            }
        },
    )?;

    // Input
    linker.func_wrap("env", "get_mouse_x", |_: ProcessCaller| unsafe {
        KERNEL.mousestate.x as i32
    })?;

    linker.func_wrap("env", "get_mouse_y", |_: ProcessCaller| unsafe {
        KERNEL.mousestate.y as i32
    })?;

    // Process

    linker.func_wrap_async("env", "yield_now", |_: ProcessCaller, _: ()| {
        Box::new(async {
            yield_now().await;
        })
    })?;

    Ok(())
}

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
                KERNEL.set_current_pid(self_cell.get().store.data().pid());
            }

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
        std::mem::swap(&mut old_event_queue, &mut self.store.data_mut().event_queue);

        for event in old_event_queue {
            self.process_event(event);
        }
    }

    fn process_event(&mut self, mut event: Event) {
        unsafe {
            KERNEL.set_current_event(&raw mut event);
            let self_ptr = self as *mut Self;

            let sym = event.interned_name;
            if let Some(handler) = (*self_ptr).store.data().event_handlers.get(&sym) {
                let result = handler.call(&mut self.store, &[], &mut []);
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
