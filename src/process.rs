#![allow(static_mut_refs)]

use std::{
    any::Any,
    collections::HashMap,
    io,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    sync::Arc,
    time::UNIX_EPOCH,
};

use string_interner::symbol::SymbolU32;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::task::JoinHandle;
use wasmtime::ModuleExport;

use crate::{
    async_file::AsyncFile,
    cell::mut_cell::MutCell,
    cell::ptr_cell::PtrCell,
    event::Event,
    graphics::graphics_state::GraphicsState,
    id::{Id, IdStore},
    kernel::{FILE_CREATE, FILE_WRITE, Kernel, Pid},
    wasm_process::WasmProcess,
};

type Environment = HashMap<Box<[u8]>, Box<[u8]>>;

pub enum PreparedData {
    None,
    Arg(u16),
    Bytes(Box<[u8]>),
    Label,
    PidLabel(Pid),
    Cwd,
}

impl PreparedData {
    pub fn take(&mut self) -> Self {
        std::mem::replace(self, Self::None)
    }
}

/// A process represents the state of a running Webassembly process.
/// This is just the data associated with it.
/// It is used in system functions.
pub struct Process<T: 'static> {
    /// Backreference to the kernel that owns this process.
    pub kernel: &'static MutCell<Kernel<T>>,
    /// This process' id
    pub pid: Pid,
    /// The parent process' id
    pub parent_pid: Pid,
    /// The list of arguments given to the process
    pub args: Box<[Box<[u8]>]>,
    /// The process' environment
    pub environment: Environment,
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
    pub open_files: IdStore<Arc<MutCell<AsyncFile>>>,
    /// The table of iterated directories.
    pub directory_iterators: IdStore<cap_std::fs::ReadDir>,
    /// Can be awaited to end the process early.
    pub join_handle: Option<JoinHandle<i32>>,
    /// The eventual return value of the process.
    /// Currently has no use.
    pub exit_code: Arc<MutCell<Option<u16>>>,
    /// The list of children.
    /// TODO: update this when a child exits.
    pub children: Vec<Pid>,
    pub child_iter_index: Option<u32>,
    /// Events that are yet to be handled.
    pub event_queue: Vec<Event>,
    /// Wasm functions to handle the events.
    pub event_handlers: HashMap<SymbolU32, wasmtime::TypedFunc<i32, ()>>,
    /// The default wasm function for any event
    pub default_event_handler: Option<wasmtime::TypedFunc<i32, ()>>,
    pub data: PreparedData,
    pub graphics_state: GraphicsState,
    pub driver_states: HashMap<usize, Box<dyn Any + Send>>,
}

impl<T> Process<T> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kernel: &'static MutCell<Kernel<T>>,
        parent_pid: Pid,
        args: Box<[Box<[u8]>]>,
        environment: Environment,
        label: impl Into<Box<str>>,
        current_working_directory: PathBuf,
        stdin: Arc<MutCell<AsyncFile>>,
        stdout: Arc<MutCell<AsyncFile>>,
        stderr: Arc<MutCell<AsyncFile>>,
    ) -> Self {
        let mut open_files = IdStore::new();
        open_files.new_id(stdin);
        open_files.new_id(stdout);
        open_files.new_id(stderr);

        Self {
            kernel,
            pid: Id::new(0),
            parent_pid,
            args,
            environment,
            label: label.into(),
            current_working_directory,
            memory_export: None,
            open_files,
            directory_iterators: IdStore::new(),
            join_handle: None,
            exit_code: Arc::new(MutCell::new(None)),
            children: Vec::new(),
            child_iter_index: None,
            event_queue: Vec::new(),
            event_handlers: HashMap::new(),
            default_event_handler: None,
            data: PreparedData::None,
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

    pub fn prepare_arg(&mut self, index: u16) -> usize {
        self.data = PreparedData::Arg(index);
        self.args[index as usize].len()
    }

    pub fn prepare_var(&mut self, key: &[u8]) -> usize {
        let self_cell = PtrCell::new(self);
        if let Some(value) = self_cell.get().environment.get(key) {
            self.prepare_bytes(value)
        } else {
            self.data = PreparedData::None;
            0
        }
    }

    pub fn set_var(&mut self, key: &[u8], value: &[u8]) {
        self.environment.insert(
            key.to_owned().into_boxed_slice(),
            value.to_owned().into_boxed_slice(),
        );
    }

    pub fn delete_var(&mut self, key: &[u8]) -> bool {
        self.environment.remove(key).is_some()
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

        self.open_files
            .new_id(Arc::new(MutCell::new(AsyncFile::File(file))))
    }

    pub fn foo_file(&self, fd: Id) -> Arc<MutCell<AsyncFile>> {
        self.open_files
            .data(fd)
            .cloned()
            .unwrap_or_else(|| Arc::new(MutCell::new(AsyncFile::Null)))
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
        if self.kernel.is_directory(path) {
            self.current_working_directory = path.into();
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

    pub fn get_file(&mut self, fd: Id) -> Option<&mut Arc<MutCell<AsyncFile>>> {
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

    pub fn set_default_handler(&mut self, handler: wasmtime::TypedFunc<i32, ()>) {
        self.default_event_handler = Some(handler);
    }

    // Data

    pub fn prepare_bytes(&mut self, bytes: &[u8]) -> usize {
        self.data = PreparedData::Bytes(bytes.to_owned().into_boxed_slice());
        self.get_data_length()
    }

    pub fn prepare_label(&mut self) -> usize {
        self.data = PreparedData::Label;
        self.get_data_length()
    }

    pub fn prepare_process_label(&mut self, pid: Pid) -> usize {
        self.data = PreparedData::PidLabel(pid);
        self.get_data_length()
    }

    pub fn prepare_cwd(&mut self) -> usize {
        self.data = PreparedData::Cwd;
        self.get_data_length()
    }

    pub fn get_data_length(&self) -> usize {
        self.data_to_bytes().len()
    }

    pub fn data_to_bytes(&self) -> &[u8] {
        match &self.data {
            PreparedData::None => &[],
            PreparedData::Bytes(b) => b,
            PreparedData::Arg(i) => &self.args[*i as usize],
            PreparedData::Label => self.label.as_bytes(),
            PreparedData::PidLabel(pid) => {
                if let Some(process) = self.kernel.get_process(*pid) {
                    process.store.data().label.as_bytes()
                } else {
                    &[]
                }
            }
            PreparedData::Cwd => self.current_working_directory.as_os_str().as_bytes(),
        }
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

    /// Gets the entire wasm memory as a slice.
    pub fn get_entire_memory(&self) -> &'static mut [u8] {
        self.as_wasm_process().get_entire_memory()
    }

    /// Gets the memory address of the current draw region.
    pub fn get_draw_address(&self) -> *mut u8 {
        self.get_memory(
            self.graphics_state.draw_address,
            self.graphics_state.draw_region.area() as usize,
        )
        .as_mut_ptr()
    }

    pub fn assert_memory_size(&self, address: usize, len: usize, mem_name: &str) {
        assert!(
            self.get_entire_memory()
                .get(address..(address + len))
                .is_some(),
            "Expected memory slice '{mem_name}' to be within bounds."
        );
    }
}

fn time_since_unix_epoch(time: cap_std::time::SystemTime) -> i64 {
    time.into_std()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(-1)
}
