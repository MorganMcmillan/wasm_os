#![allow(static_mut_refs)]

use std::any::Any;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::task::JoinHandle;

use string_interner::symbol::SymbolU32;

use crate::KERNEL;
use crate::async_file::AsyncFile;
use crate::event::Event;
use crate::id::{Id, IdStore};
use crate::kernel::Pid;

/// A process represents the state of a running Webassembly process.
pub struct Process {
    pub pid: Pid,
    pub parent_pid: Pid,
    pub label: Box<str>,
    pub current_working_directory: PathBuf,
    pub open_files: IdStore<AsyncFile>,
    pub join_handle: Option<JoinHandle<i32>>,
    pub exit_code: Option<u16>,
    pub children: Vec<Pid>,
    pub event_queue: Vec<Event>,
    pub event_handlers: HashMap<SymbolU32, wasmtime::TypedFunc<i32, ()>>,
    pub driver_states: HashMap<usize, Box<dyn Any + Send>>,
}

#[allow(dead_code)]
impl Process {
    pub fn new(
        parent_pid: Pid,
        label: impl Into<Box<str>>,
        current_working_directory: PathBuf,
        stdin: AsyncFile,
        stdout: AsyncFile,
        stderr: AsyncFile,
    ) -> Self {
        let mut open_files = IdStore::new();
        open_files.new_id(stdin);
        open_files.new_id(stdout);
        open_files.new_id(stderr);

        Self {
            pid: Id::new(0),
            parent_pid,
            label: label.into(),
            current_working_directory,
            open_files,
            join_handle: None,
            exit_code: None,
            children: Vec::new(),
            event_queue: Vec::new(),
            event_handlers: HashMap::new(),
            driver_states: HashMap::new(),
        }
    }

    pub fn set_pid(&mut self, pid: Pid) {
        self.pid = pid;
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

    // Files

    pub fn read_whole_file(&self, path: impl AsRef<Path>) -> Option<Vec<u8>> {
        let mut cwd = self.current_working_directory.clone();
        cwd.push(path);
        unsafe { KERNEL.read_file(&cwd).ok() }
    }

    fn set_current_directory(&mut self, path: &Path) -> i32 {
        unsafe {
            // TODO: do we even need this? Surely the relative path should resolve to a file.
            let Ok(path) = KERNEL.get_absolute_path(path) else {
                return -2;
            };

            if KERNEL.directory_exists(&path) {
                self.current_working_directory = path;
                0
            } else {
                -1
            }
        }
    }

    // Gets the absolute path relative to this process' current working directory.
    fn get_absolute_path(&self, path: &Path) -> PathBuf {
        if let Ok(abs_path) = path.strip_prefix("/") {
            abs_path.to_path_buf()
        } else {
            let mut relative_path = self.current_working_directory.clone();
            relative_path.push(path);
            relative_path
        }
    }

    pub fn change_directory(&mut self, path: impl AsRef<Path>) -> i32 {
        self.set_current_directory(&self.get_absolute_path(path.as_ref()))
    }

    // TODO: add `open_file`

    // Events

    pub fn push_event(&mut self, event: Event) {
        self.event_queue.push(event);
    }

    pub fn add_event_handler(&mut self, name: SymbolU32, handler: wasmtime::TypedFunc<i32, ()>) {
        self.event_handlers.insert(name, handler);
    }

    pub fn remove_event_handler(&mut self, name: SymbolU32) {
        self.event_handlers.remove(&name);
    }

    pub fn add_driver_state(&mut self, driver_id: usize, state: Box<dyn Any + Send>) {
        self.driver_states.insert(driver_id, state);
    }

    pub fn get_driver_state<T: Any + Send>(&self, driver_id: usize) -> Option<&T> {
        self.driver_states
            .get(&driver_id)
            .and_then(|state| state.downcast_ref::<T>())
    }

    pub fn get_driver_state_mut<T: Any + Send>(&mut self, driver_id: usize) -> Option<&mut T> {
        self.driver_states
            .get_mut(&driver_id)
            .and_then(|state| state.downcast_mut::<T>())
    }
}
