#![allow(static_mut_refs)]

use std::any::Any;
use std::collections::HashMap;
use tokio::task::JoinHandle;

use string_interner::symbol::SymbolU32;
use wasmtime::TypedFunc;
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxView, WasiView};

use crate::event::Event;
use crate::kernel::Pid;

/// A process represents the state of a running Webassembly process.
pub struct Process {
    pub pid: Pid,
    pub parent_pid: Pid,
    pub exit_code: Option<u16>,
    pub children: Vec<Pid>,
    pub event_queue: Vec<Event>,
    pub event_handlers: HashMap<SymbolU32, TypedFunc<(i32,), ()>>,
    pub join_handle: Option<JoinHandle<i32>>,
    pub label: Box<str>,
    pub driver_states: HashMap<usize, Box<dyn Any + Send>>,
    pub wasi_ctx: WasiCtx,
    pub wasi_table: ResourceTable,
}

#[allow(dead_code)]
impl Process {
    pub fn new(wasi_ctx: WasiCtx, pid: Pid, parent_pid: Pid, label: impl Into<Box<str>>) -> Self {
        Self {
            pid,
            parent_pid,
            exit_code: None,
            children: Vec::new(),
            event_queue: Vec::new(),
            event_handlers: HashMap::new(),
            join_handle: None,
            label: label.into(),
            driver_states: HashMap::new(),
            wasi_ctx,
            wasi_table: ResourceTable::new(),
        }
    }

    pub fn add_child(&mut self, pid: Pid) {
        self.children.push(pid);
    }

    pub fn set_join_handle(&mut self, join_handle: JoinHandle<i32>) {
        if self.join_handle.is_some() {
            panic!("Cannot set join handle of a process when it is already set!");
        }
        self.join_handle = Some(join_handle);
    }

    pub async fn kill(&mut self) {
        let _ = self.join_handle.as_mut().unwrap().await;
    }

    // Events

    pub fn push_event(&mut self, event: Event) {
        self.event_queue.push(event);
    }

    pub fn add_event_handler(&mut self, name: SymbolU32, handler: TypedFunc<(i32,), ()>) {
        self.event_handlers.insert(name, handler);
    }

    pub fn remove_event_handler(&mut self, name: SymbolU32) {
        self.event_handlers.remove(&name);
    }
}

impl WasiView for Process {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi_ctx,
            table: &mut self.wasi_table,
        }
    }
}
