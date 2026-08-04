#![allow(static_mut_refs)]

use std::any::Any;
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::task::JoinHandle;

use string_interner::symbol::SymbolU32;
use wasmtime::ModuleExport;

use crate::async_file::AsyncFile;
use crate::event::Event;
use crate::graphics::graphics_state::GraphicsState;
use crate::id::{Id, IdStore};
use crate::kernel::{FILE_CREATE, FILE_WRITE, Kernel, Pid};
use crate::mut_cell::MutCell;
use crate::wasm_process::WasmProcess;

/// A process represents the state of a running Webassembly process.
pub struct Process<T: 'static> {
    pub kernel: &'static MutCell<Kernel<T>>,
    /// This process' id
    pub pid: Pid,
    /// The parent process' id
    pub parent_pid: Pid,
    /// The label is this process' file name without the extension
    pub label: Box<str>,
    /// The directory for which files are opened relative to
    pub current_working_directory: PathBuf,
    /// The exported index of the process's memory
    pub memory_export: Option<ModuleExport>,
    /// The table of open files.
    /// The following files always have these ids:
    /// Stdin: (1, 0),
    /// Stdout: (2, 0),
    /// Stderr: (3, 0)
    pub open_files: IdStore<AsyncFile>,
    pub directory_iterators: IdStore<cap_std::fs::ReadDir>,
    /// Can be awaited to end the process early
    pub join_handle: Option<JoinHandle<i32>>,
    /// The return value of the process
    pub exit_code: Option<u16>,
    pub children: Vec<Pid>,
    pub child_iter_index: Option<u32>,
    pub event_queue: Vec<Event>,
    pub event_handlers: HashMap<SymbolU32, wasmtime::TypedFunc<i32, ()>>,
    pub default_event_handler: Option<wasmtime::TypedFunc<(i32, i32), ()>>,
    pub byte_data: Option<Vec<u8>>,
    pub graphics_state: GraphicsState,
    pub driver_states: HashMap<usize, Box<dyn Any + Send>>,
}

impl<T> Process<T> {
    pub fn new(
        kernel: &'static MutCell<Kernel<T>>,
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
            kernel,
            pid: Id::new(0),
            parent_pid,
            label: label.into(),
            current_working_directory,
            memory_export: None,
            open_files,
            directory_iterators: IdStore::new(),
            join_handle: None,
            exit_code: None,
            children: Vec::new(),
            child_iter_index: None,
            event_queue: Vec::new(),
            event_handlers: HashMap::new(),
            default_event_handler: None,
            byte_data: Some(Vec::with_capacity(256)),
            graphics_state: GraphicsState::new(),
            driver_states: HashMap::new(),
        }
    }

    pub fn set_pid(&mut self, pid: Pid) {
        self.pid = pid;
    }

    pub fn add_child(&mut self, pid: Pid) {
        self.children.push(pid);
    }

    pub fn iter_children(&mut self) {
        self.child_iter_index = Some(0);
    }

    pub fn next_child(&mut self) -> Pid {
        if let Some(index) = &mut self.child_iter_index {
            if let Some(child) = self.children.get(*index as usize) {
                *child
            } else {
                self.child_iter_index = None;
                Pid::default()
            }
        } else {
            Pid::default()
        }
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

    pub fn is_directory(&self, path: impl AsRef<Path>) -> bool {
        self.kernel
            .is_directory(self.get_absolute_path(path.as_ref()))
    }

    pub fn is_file(&self, path: impl AsRef<Path>) -> bool {
        self.kernel.is_file(self.get_absolute_path(path.as_ref()))
    }

    pub fn file_exists(&self, path: impl AsRef<Path>) -> bool {
        self.kernel
            .file_exists(self.get_absolute_path(path.as_ref()))
    }

    pub fn file_size(&self, path: impl AsRef<Path>) -> i32 {
        match self.kernel.file_size(path) {
            Ok(size) => size as i32,
            Err(_) => -1,
        }
    }

    pub fn file_created(&self, path: impl AsRef<Path>) -> i64 {
        self.kernel
            .file_created(path)
            .map(time_since_unix_epoch)
            .unwrap_or(-1)
    }

    pub fn file_accessed(&self, path: impl AsRef<Path>) -> i64 {
        self.kernel
            .file_accessed(path)
            .map(time_since_unix_epoch)
            .unwrap_or(-1)
    }

    pub fn file_modified(&self, path: impl AsRef<Path>) -> i64 {
        self.kernel
            .file_modified(path)
            .map(time_since_unix_epoch)
            .unwrap_or(-1)
    }

    pub fn open_file(&mut self, path: impl AsRef<Path>, mode: i32) -> Id {
        let path = self.get_absolute_path(path.as_ref());

        let file = match self
            .kernel
            .borrow_static()
            .open_async_file(&path, mode as u8)
        {
            Ok(f) => f,
            Err(_) => return Id::default(),
        };

        self.open_files.new_id(AsyncFile::File(file))
    }

    pub async fn read_entire_file(&mut self, path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
        let path = self.get_absolute_path(path.as_ref());

        let mut file = self.kernel.borrow_static().open_async_file(&path, 0)?;

        let mut contents = Vec::with_capacity(64);

        file.read_to_end(&mut contents).await?;

        Ok(contents)
    }

    pub async fn write_entire_file(
        &mut self,
        path: impl AsRef<Path>,
        contents: &[u8],
    ) -> io::Result<()> {
        let path = self.get_absolute_path(path.as_ref());

        let mut file = self
            .kernel
            .borrow_static()
            .open_async_file(&path, FILE_WRITE | FILE_CREATE)?;

        file.write_all(contents).await
    }

    fn set_current_directory(&mut self, path: &Path) -> i32 {
        let Ok(path) = self.kernel.get_absolute_path(path) else {
            return -2;
        };

        if self.kernel.is_directory(&path) {
            self.current_working_directory = path;
            0
        } else {
            -1
        }
    }
    pub fn change_directory(&mut self, path: impl AsRef<Path>) -> i32 {
        self.set_current_directory(&self.get_absolute_path(path.as_ref()))
    }

    pub fn move_file(&mut self, from: impl AsRef<Path>, to: impl AsRef<Path>) -> i32 {
        if self
            .kernel
            .move_file(
                self.get_absolute_path(from.as_ref()),
                self.get_absolute_path(to.as_ref()),
            )
            .is_ok()
        {
            0
        } else {
            -1
        }
    }

    pub fn copy_file(&mut self, from: impl AsRef<Path>, to: impl AsRef<Path>) -> i32 {
        if self
            .kernel
            .copy_file(
                self.get_absolute_path(from.as_ref()),
                self.get_absolute_path(to.as_ref()),
            )
            .is_ok()
        {
            0
        } else {
            -1
        }
    }

    pub fn delete_file(&mut self, path: impl AsRef<Path>) -> i32 {
        if self
            .kernel
            .delete_file(self.get_absolute_path(path.as_ref()))
            .is_ok()
        {
            0
        } else {
            -1
        }
    }

    pub fn create_directory(&mut self, path: impl AsRef<Path>) -> i32 {
        if self
            .kernel
            .create_directory(self.get_absolute_path(path.as_ref()))
            .is_ok()
        {
            0
        } else {
            -1
        }
    }

    pub fn get_file(&mut self, fd: Id) -> Option<&mut AsyncFile> {
        self.open_files.data_mut(fd)
    }

    pub fn iter_directory(&mut self, path: impl AsRef<Path>) -> Id {
        let Ok(iter) = self.kernel.read_directory(path) else {
            return Id::default();
        };

        self.directory_iterators.new_id(iter)
    }

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

    pub fn set_default_handler(&mut self, handler: wasmtime::TypedFunc<(i32, i32), ()>) {
        self.default_event_handler = Some(handler);
    }

    // Data

    pub fn set_data(&mut self, data: &[u8]) -> usize {
        self.byte_data = Some(data.into());
        data.len()
    }

    // Drivers

    pub fn add_driver_state(&mut self, driver_id: usize, state: Box<dyn Any + Send>) {
        self.driver_states.insert(driver_id, state);
    }

    #[allow(unused)]
    pub fn get_driver_state<D: Any + Send>(&self, driver_id: usize) -> Option<&D> {
        self.driver_states
            .get(&driver_id)
            .and_then(|state| state.downcast_ref::<D>())
    }

    pub fn get_driver_state_mut<D: Any + Send>(&mut self, driver_id: usize) -> Option<&mut D> {
        self.driver_states
            .get_mut(&driver_id)
            .and_then(|state| state.downcast_mut::<D>())
    }

    // Wasm-side stuff

    pub fn as_wasm_process(&self) -> &mut WasmProcess<T> {
        self.kernel
            .borrow_static()
            .get_process_mut(self.pid)
            .unwrap()
    }

    /// Gets a mutable slice of memory
    pub fn get_memory(&self, address: usize, len: usize) -> &'static mut [u8] {
        self.as_wasm_process().get_memory(address, len)
    }

    pub fn set_memory(&self, address: usize, value: &[u8]) {
        self.as_wasm_process().set_memory(address, value);
    }

    /// Gets the memory address of the current draw region.
    pub fn get_draw_address(&self) -> *mut u8 {
        self.get_memory(
            self.graphics_state.draw_address,
            self.graphics_state.draw_region.area() as usize,
        )
        .as_mut_ptr()
    }
}

fn time_since_unix_epoch(time: cap_std::time::SystemTime) -> i64 {
    time.into_std()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(-1)
}
