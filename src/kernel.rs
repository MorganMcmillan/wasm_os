use std::any::Any;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::io;
use std::io::ErrorKind::NotFound;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::ptr::NonNull;
use std::time::Duration;

use cap_std::ambient_authority;
use cap_std::fs::MetadataExt;
use cap_std::fs::OpenOptions;
use cap_std::time::SystemTime;
use raylib::RaylibHandle;
use raylib::ffi::KeyboardKey;
use string_interner::StringInterner;
use string_interner::backend::StringBackend;
use string_interner::symbol::SymbolU32;
use tokio::task;
use wasmtime::Caller;
use wasmtime::Config;
use wasmtime::Engine;

use crate::async_file::AsyncFile;
use crate::driver::Driver;
use crate::event::Event;
use crate::id::Id;
use crate::id::IdStore;
use crate::mut_cell::MutCell;
use crate::process::Process;
use crate::wasm_process::WasmProcess;

pub type Pid = Id;
pub type ProcessLinker = wasmtime::Linker<Process>;
pub type ProcessContext<'a> = Caller<'a, Process>;

pub const ROOT_PID: Pid = Id::new(1);

#[derive(Debug)]
pub enum CreateProcessError {
    FileNotFound,
    InvalidWasm,
    IncorrectFileType,
    Other,
}

pub struct Kernel {
    engine: Engine,
    pub drivers: Vec<Box<dyn Driver>>,
    // A sparse map of Pids to processes
    pub processes: IdStore<WasmProcess>,
    // A map of process names to Pids
    process_names: HashMap<Box<str>, Pid>,
    current_event: Option<NonNull<Event>>,
    interned_event_names: StringInterner<StringBackend>,
    // The top-level directory for which programs can be executed from
    ambient_dir: cap_std::fs::Dir,
}

unsafe impl Send for Kernel {}
unsafe impl Sync for Kernel {}

const BIOS_BOOT_PROCESS: &str = "bios/boot.wasm";
const ROM_BOOT_PROCESS: &str = "rom/boot.wasm";
const USER_BOOT_PROCESS: &str = "boot.wasm";

fn create_system_folders(root_dir: &cap_std::fs::Dir) {
    let _ = root_dir.create_dir("bios");
    let _ = root_dir.create_dir("rom");
    let _ = root_dir.create_dir("lib");
}

impl Kernel {
    pub fn new(root_dir: &Path, drivers: Vec<Box<dyn Driver>>) -> Self {
        let mut config = Config::new();
        config.strategy(wasmtime::Strategy::Cranelift);
        config.epoch_interruption(true);

        let engine = Engine::new(&config).unwrap();
        let engine_clone = engine.weak();

        // Periodically interupt process execution
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(50));
                if let Some(engine) = engine_clone.upgrade() {
                    engine.increment_epoch();
                } else {
                    break;
                }
            }
        });

        let ambient_dir =
            cap_std::fs::Dir::open_ambient_dir(root_dir, ambient_authority()).unwrap();

        create_system_folders(&ambient_dir);

        Self {
            engine,
            drivers,
            processes: IdStore::new(),
            process_names: HashMap::new(),
            current_event: None,
            interned_event_names: StringInterner::new(),
            ambient_dir,
        }
    }

    pub fn root_exited(&self) -> bool {
        !self.processes.id_is_valid(ROOT_PID)
    }

    pub async fn run_boot(kernel: &'static MutCell<Kernel>) {
        let root_pid = Kernel::create_boot_process(kernel).await;
        let root = kernel.borrow_static().get_process_mut(root_pid).unwrap();
        let join_handle = task::spawn(root.run());

        kernel
            .borrow_static()
            .get_process_mut(root_pid)
            .unwrap()
            .store
            .data_mut()
            .set_join_handle(join_handle);
    }

    async fn create_boot_process(kernel: &'static MutCell<Kernel>) -> Pid {
        async fn create_boot_process(
            kernel: &'static MutCell<Kernel>,
            path: &str,
        ) -> Result<Pid, CreateProcessError> {
            Kernel::create_process(
                kernel,
                path,
                Pid::default(),
                AsyncFile::stdin(),
                AsyncFile::stdout(),
                AsyncFile::stderr(),
            )
            .await
        }

        let root_process = match create_boot_process(kernel, USER_BOOT_PROCESS).await {
            Err(CreateProcessError::FileNotFound) => {
                match create_boot_process(kernel, ROM_BOOT_PROCESS).await {
                    Err(CreateProcessError::FileNotFound) => {
                        create_boot_process(kernel, BIOS_BOOT_PROCESS).await
                    }
                    result => result,
                }
            }
            result => result,
        };

        match root_process {
            Ok(r) => r,
            Err(e) => panic!(
                "Could not create boot process for any 'boot.wasm': {:?}.",
                e
            ),
        }
    }

    pub async fn run_process(
        kernel: &'static MutCell<Kernel>,
        path: &str,
        parent: Pid,
        stdin: AsyncFile,
        stdout: AsyncFile,
        stderr: AsyncFile,
    ) -> Result<Pid, CreateProcessError> {
        let pid = Kernel::create_process(kernel, path, parent, stdin, stdout, stderr).await?;
        let process = kernel.borrow_static().get_process_mut(pid).unwrap();
        let join_handle = task::spawn(process.run());

        kernel
            .borrow_static()
            .get_process_mut(pid)
            .unwrap()
            .store
            .data_mut()
            .set_join_handle(join_handle);

        println!("Spawned process '{path}'");

        Ok(pid)
    }

    // Files

    pub fn is_directory(&self, path: impl AsRef<Path>) -> bool {
        self.ambient_dir.is_dir(path)
    }

    pub fn is_file(&self, path: impl AsRef<Path>) -> bool {
        self.ambient_dir.is_file(path)
    }

    pub fn file_exists(&self, path: impl AsRef<Path>) -> bool {
        self.ambient_dir.exists(path)
    }

    pub fn get_absolute_path(&self, path: impl AsRef<Path>) -> io::Result<PathBuf> {
        self.ambient_dir.canonicalize(path)
    }

    pub fn file_size(&self, path: impl AsRef<Path>) -> io::Result<u64> {
        self.ambient_dir.metadata(path).map(|md| md.size())
    }

    pub fn file_created(&self, path: impl AsRef<Path>) -> io::Result<SystemTime> {
        self.ambient_dir.metadata(path).and_then(|md| md.created())
    }

    pub fn file_accessed(&self, path: impl AsRef<Path>) -> io::Result<SystemTime> {
        self.ambient_dir.metadata(path).and_then(|md| md.accessed())
    }

    pub fn file_modified(&self, path: impl AsRef<Path>) -> io::Result<SystemTime> {
        self.ambient_dir.metadata(path).and_then(|md| md.modified())
    }

    pub fn open_async_file(
        &self,
        path: impl AsRef<Path>,
        mode: u8,
    ) -> Result<tokio::fs::File, io::Error> {
        const OPTION_WRITE: u8 = 0b1;
        const OPTION_APPEND: u8 = 0b10;
        const OPTION_CREATE: u8 = 0b100;
        const OPTION_TRUNCATE: u8 = 0b1000;

        let mut options = OpenOptions::new();
        if mode & OPTION_WRITE != 0 {
            options.write(true);
        } else {
            options.read(true);
        }
        if mode & OPTION_APPEND != 0 {
            options.append(true);
        }
        if mode & OPTION_CREATE != 0 {
            options.create(true);
        }
        if mode & OPTION_TRUNCATE != 0 {
            options.truncate(true);
        }

        let file = self.ambient_dir.open_with(path, &options)?;
        Ok(tokio::fs::File::from_std(file.into_std()))
    }

    pub fn read_file(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, CreateProcessError> {
        let mut file = match self.ambient_dir.open(path.as_ref()) {
            Ok(f) => f,
            Err(e) => match e.kind() {
                NotFound => return Err(CreateProcessError::FileNotFound),
                _ => return Err(CreateProcessError::Other),
            },
        };

        let mut bytes = Vec::with_capacity(1 << 12);
        if file.read_to_end(&mut bytes).is_err() {
            return Err(CreateProcessError::Other);
        }

        Ok(bytes)
    }

    pub fn move_file(&self, from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
        self.ambient_dir.rename(from, &self.ambient_dir, to)
    }

    pub fn copy_file(&self, from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<u64> {
        self.ambient_dir.copy(from, &self.ambient_dir, to)
    }

    pub fn delete_file(&self, path: impl AsRef<Path>) -> io::Result<()> {
        self.ambient_dir
            .remove_file(&path)
            .or_else(|_| self.ambient_dir.remove_dir_all(&path))
    }

    pub fn create_directory(&self, path: impl AsRef<Path>) -> io::Result<()> {
        self.ambient_dir.create_dir(path)
    }

    #[allow(unused)]
    pub fn create_file(&self, path: impl AsRef<Path>) -> io::Result<()> {
        self.ambient_dir.create(path).map(|_| ())
    }

    pub async fn create_process(
        kernel: &'static MutCell<Kernel>,
        path: &str,
        parent: Pid,
        stdin: AsyncFile,
        stdout: AsyncFile,
        stderr: AsyncFile,
    ) -> Result<Pid, CreateProcessError> {
        if !path.ends_with(".wasm") {
            return Err(CreateProcessError::IncorrectFileType);
        }

        let binary = kernel.read_file(path)?;
        let path = std::path::Path::new(path);
        let label = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("_UNKNOWN_PROGRAM");

        let cwd = if parent.number() == 0 {
            PathBuf::new()
        } else {
            kernel
                .get_process(parent)
                .unwrap()
                .store
                .data()
                .current_working_directory
                .clone()
        };

        let mut process = Process::new(kernel, parent, label, cwd, stdin, stdout, stderr);

        for (id, driver) in kernel.borrow_static().drivers.iter_mut().enumerate() {
            if let Some(process_state) = driver.create_process_state() {
                process.add_driver_state(id, process_state);
            }
        }

        let wasm_process = match WasmProcess::new(kernel, binary, &kernel.engine, process).await {
            Ok(w) => w,
            Err(e) => {
                eprintln!("Wasm error: {:?}", e);
                return Err(CreateProcessError::InvalidWasm);
            }
        };

        let pid = kernel.borrow_static().processes.new_id(wasm_process);

        kernel
            .borrow_static()
            .get_process_mut(pid)
            .unwrap()
            .store
            .data_mut()
            .set_pid(pid);

        // If not root process:
        if parent.number() != 0 {
            kernel
                .borrow_static()
                .get_process_mut(parent)
                .expect("Expected a parent process to exist.")
                .store
                .data_mut()
                .add_child(pid);
        }

        Ok(pid)
    }

    pub fn delete_process(&mut self, pid: Pid) {
        self.processes.delete_id(pid);
    }

    pub fn get_process(&self, pid: Pid) -> Option<&WasmProcess> {
        self.processes.data(pid)
    }

    pub fn get_process_mut(&mut self, pid: Pid) -> Option<&mut WasmProcess> {
        self.processes.data_mut(pid)
    }

    pub fn update(
        kernel: &'static MutCell<Kernel>,
        rl: &mut RaylibHandle,
        thread: &raylib::RaylibThread,
    ) {
        if rl.is_key_pressed(KeyboardKey::KEY_F11) {
            rl.toggle_fullscreen();
        }

        for driver in kernel.borrow_static().drivers.iter_mut() {
            driver.update(kernel, rl, thread);
        }
    }

    // Drivers

    pub fn get_driver_by_name(&mut self, name: &str) -> Option<(usize, &mut dyn Driver)> {
        for (id, driver) in self.drivers.iter_mut().enumerate() {
            if driver.name() == name {
                return Some((id, driver.as_mut()));
            }
        }
        None
    }

    // Loads each driver's functions.
    pub fn load_driver_functions(
        &mut self,
        linker: &mut ProcessLinker,
        imported_modules: &[&str],
    ) -> wasmtime::Result<()> {
        for driver_name in imported_modules.iter().copied() {
            if let Some((id, driver)) = self.get_driver_by_name(driver_name) {
                driver.register_functions(linker, id)?;
            }
        }

        Ok(())
    }

    pub fn get_driver<T: Any>(&mut self, id: usize) -> &mut T {
        self.drivers[id].as_any().downcast_mut().unwrap()
    }

    // Events

    pub fn intern_event_name(&mut self, name: &str) -> SymbolU32 {
        self.interned_event_names.get_or_intern(name)
    }

    pub fn send_event(
        &mut self,
        event_name: &str,
        event_data: &[u8],
        sender: Pid,
        receiver: Pid,
    ) -> i32 {
        let interned_name = self.interned_event_names.get_or_intern(event_name);
        let copied_data = event_data.to_owned().into_boxed_slice();

        let event = Event::new(copied_data, sender, interned_name);

        if let Some(receiver_process) = self.get_process_mut(receiver) {
            receiver_process.store.data_mut().push_event(event);
            0
        } else {
            -1
        }
    }

    /// Sends an event to the root process.
    /// Meant to be used by input drivers.
    pub fn send_event_to_root(&mut self, event_name: &str, event_data: &[u8]) -> i32 {
        self.send_event(event_name, event_data, Id::default(), ROOT_PID)
    }

    pub fn resend_event(&mut self, event: &Event, sender: Pid, receiver: Pid) -> i32 {
        if let Some(receiver_process) = self.get_process_mut(receiver) {
            let event = Event::from_resent(event, sender);
            receiver_process.store.data_mut().push_event(event);
            0
        } else {
            -1
        }
    }

    pub fn set_current_event(&mut self, event_ptr: *mut Event) {
        self.current_event = NonNull::new(event_ptr);
    }

    pub fn end_current_event(&mut self) {
        self.current_event = None;
    }

    pub fn get_current_event(&self) -> &Event {
        unsafe {
            self.current_event
                .expect("Cannot get event data when outside of event handler or when handler is called without an event.")
                .as_ref()
        }
    }

    pub fn get_event_name(&self, interned_name: SymbolU32) -> &str {
        self.interned_event_names
            .resolve(interned_name)
            .unwrap_or("NO_EVENT_NAME")
    }

    // Proceses

    pub fn set_process_name(&mut self, pid: Pid, name: &str) -> bool {
        match self.process_names.entry(name.into()) {
            Entry::Vacant(entry) => {
                entry.insert(pid);
                true
            }
            Entry::Occupied(_) => false,
        }
    }

    pub fn get_pid_by_name(&self, name: &str) -> Pid {
        self.process_names
            .get(name)
            .copied()
            .unwrap_or_else(Pid::default)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn move_file_works() -> io::Result<()> {
        let root_dir = "test_dir";
        let kernel = Kernel::new(root_dir.as_ref(), vec![]);

        let _ = kernel.create_directory("foo");
        kernel.move_file("foo", "bar").unwrap();
        assert!(kernel.is_directory("bar"));
        assert!(!kernel.file_exists("foo"));
        kernel.delete_file("bar").unwrap();

        Ok(())
    }

    #[test]
    fn copy_file_works() -> io::Result<()> {
        let root_dir = "test_dir";
        let kernel = Kernel::new(root_dir.as_ref(), vec![]);

        kernel.create_file("foo").unwrap();
        kernel.copy_file("foo", "bar").unwrap();
        assert!(kernel.is_file("bar"));
        assert!(kernel.is_file("foo"));
        kernel.delete_file("foo").unwrap();
        kernel.delete_file("bar").unwrap();

        Ok(())
    }
}
