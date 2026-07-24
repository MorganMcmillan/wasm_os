#![allow(static_mut_refs)]

use std::collections::HashMap;
use tokio::task::JoinHandle;

use string_interner::symbol::SymbolU32;
use wasmtime::Func;

use crate::event::Event;
use crate::kernel::Pid;

pub struct Process {
    pid: Pid,
    pub event_queue: Vec<Event>,
    pub event_handlers: HashMap<SymbolU32, Func>,
    join_handle: Option<JoinHandle<i32>>,
}

impl Process {
    pub fn new(pid: Pid) -> Self {
        Self {
            pid,
            event_queue: Vec::new(),
            event_handlers: HashMap::new(),
            join_handle: None,
        }
    }

    pub fn pid(&self) -> Pid {
        self.pid
    }

    pub fn set_join_handle(&mut self, join_handle: JoinHandle<i32>) {
        if self.join_handle.is_some() {
            panic!("Cannot set join handle of a process when it is already set!");
        }
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
}
