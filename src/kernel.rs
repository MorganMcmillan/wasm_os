use std::{
    any::Any,
    collections::{HashMap, hash_map::Entry},
    io::{self, ErrorKind::NotFound, Read},
    path::{Path, PathBuf},
    ptr::NonNull,
    time::Duration,
};

use cap_std::time::SystemTime;
use cap_std::{
    ambient_authority,
    fs::{MetadataExt, OpenOptions, ReadDir},
};
use string_interner::{StringInterner, backend::StringBackend, symbol::SymbolU32};
use tokio::task;
use wasmtime::{Caller, Config, Engine};

use crate::{
    async_file::AsyncFile,
    driver::Driver,
    event::Event,
    id::{Id, IdStore},
    mut_cell::MutCell,
    process::Process,
    wasm_process::WasmProcess,
};

pub type Pid = Id;
pub type ProcessLinker<T> = wasmtime::Linker<Process<T>>;
pub type ProcessContext<'a, T> = Caller<'a, Process<T>>;

pub const ROOT_PID: Pid = Id::new(1);

pub const FILE_READ: u8 = 0b0;
pub const FILE_WRITE: u8 = 0b1;
pub const FILE_APPEND: u8 = 0b10;
pub const FILE_CREATE: u8 = 0b100;
pub const FILE_TRUNCATE: u8 = 0b1000;

#[derive(Debug)]
pub enum CreateProcessError {
    FileNotFound,
    InvalidWasm,
    IncorrectFileType,
    Other,
}

/// A kernel is the central construct of Blaze-64.
/// It represents an instance of a Blaze-64 computer.
pub struct Kernel<T: 'static> {
    engine: Engine,
    pub drivers: Vec<Box<dyn Driver<T>>>,
    // A sparse map of Pids to processes
    pub processes: IdStore<WasmProcess<T>>,
    // A map of process names to Pids
    process_names: HashMap<Box<str>, Pid>,
    current_event: Option<NonNull<Event>>,
    interned_event_names: StringInterner<StringBackend>,
    // The top-level directory for which programs can be executed from
    ambient_dir: cap_std::fs::Dir,
}

unsafe impl<T> Send for Kernel<T> {}
unsafe impl<T> Sync for Kernel<T> {}

const BIOS_BOOT_PROCESS: &str = "bios/boot.wasm";
const ROM_BOOT_PROCESS: &str = "rom/boot.wasm";
const USER_BOOT_PROCESS: &str = "boot.wasm";

fn create_system_folders(root_dir: &cap_std::fs::Dir) {
    // TODO: replace with actually placing files inside these directories, when I figure out how to
    // do that
    let _ = root_dir.create_dir("bios");
    let _ = root_dir.create_dir("bios/bin");
    let _ = root_dir.create_dir("bios/lib");
}

impl<T: 'static> Kernel<T> {
    /// Creates a new instance mounted to `root_dir` and with `drivers` loaded.
    /// Note that this on its own doesn't do anything. You'll need to call `run_boot` to actually
    /// start up the computer.
    pub fn new(root_dir: &Path, drivers: Vec<Box<dyn Driver<T>>>) -> Self {
        let mut config = Config::new();
        config.strategy(wasmtime::Strategy::Cranelift);
        config.epoch_interruption(true);

        let engine = Engine::new(&config).unwrap();
        let engine_clone = engine.weak();

        // Create a thread to periodically preempt running processes
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(5));
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

    /// Checks if the `boot.wasm` process exited
    pub fn root_exited(&self) -> bool {
        !self.processes.id_is_valid(ROOT_PID)
    }

    /// Runs the file `boot.wasm` in either `/bios`, `/rom`, or the root directory.
    /// This is to allow users and distrubutors to have full control over their computers.
    pub async fn run_boot(kernel: &'static MutCell<Kernel<T>>) {
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

    async fn create_boot_process(kernel: &'static MutCell<Kernel<T>>) -> Pid {
        async fn create_boot_process<T>(
            kernel: &'static MutCell<Kernel<T>>,
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

    /// Executes a new wasm process.
    pub async fn run_process(
        kernel: &'static MutCell<Kernel<T>>,
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

    pub async fn create_process(
        kernel: &'static MutCell<Kernel<T>>,
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

    pub fn get_process(&self, pid: Pid) -> Option<&WasmProcess<T>> {
        self.processes.data(pid)
    }

    pub fn get_process_mut(&mut self, pid: Pid) -> Option<&mut WasmProcess<T>> {
        self.processes.data_mut(pid)
    }

    pub fn update(kernel: &'static MutCell<Self>, userdata: &mut T) {
        for driver in kernel.borrow_static().drivers.iter_mut() {
            driver.update(kernel, userdata);
        }
    }

    // Files

    /// Checks if `path` is a directory.
    pub fn is_directory(&self, path: impl AsRef<Path>) -> bool {
        self.ambient_dir.is_dir(path)
    }

    /// Checks if `path` is a file.
    pub fn is_file(&self, path: impl AsRef<Path>) -> bool {
        self.ambient_dir.is_file(path)
    }

    /// Checks if anything exists as `path`.
    pub fn file_exists(&self, path: impl AsRef<Path>) -> bool {
        self.ambient_dir.exists(path)
    }

    /// Gets the size of the file at `path`.
    pub fn file_size(&self, path: impl AsRef<Path>) -> io::Result<u64> {
        self.ambient_dir.metadata(path).map(|md| md.size())
    }

    /// Gets the time the file at `path` was created.
    pub fn file_created(&self, path: impl AsRef<Path>) -> io::Result<SystemTime> {
        self.ambient_dir.metadata(path).and_then(|md| md.created())
    }

    /// Gets the time the file at `path` was last accessed.
    pub fn file_accessed(&self, path: impl AsRef<Path>) -> io::Result<SystemTime> {
        self.ambient_dir.metadata(path).and_then(|md| md.accessed())
    }

    /// Gets the time the file at `path` was last modified.
    pub fn file_modified(&self, path: impl AsRef<Path>) -> io::Result<SystemTime> {
        self.ambient_dir.metadata(path).and_then(|md| md.modified())
    }

    /// Opens `path` as a custom [AsyncFile].
    pub fn open_async_file(
        &self,
        path: impl AsRef<Path>,
        mode: u8,
    ) -> Result<tokio::fs::File, io::Error> {
        let mut options = OpenOptions::new();
        if mode & FILE_WRITE != 0 {
            options.write(true);
        } else {
            options.read(true);
        }
        if mode & FILE_APPEND != 0 {
            options.append(true);
        }
        if mode & FILE_CREATE != 0 {
            options.create(true);
        }
        if mode & FILE_TRUNCATE != 0 {
            options.truncate(true);
        }

        let file = self.ambient_dir.open_with(path, &options)?;
        Ok(tokio::fs::File::from_std(file.into_std()))
    }

    /// Reads the entire contents of `path`.
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

    /// Moves the file/directory `from` to `to`.
    pub fn move_file(&self, from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
        self.ambient_dir.rename(from, &self.ambient_dir, to)
    }

    /// Copies the file `from` to `to`.
    pub fn copy_file(&self, from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<u64> {
        self.ambient_dir.copy(from, &self.ambient_dir, to)
    }

    /// Deletes `path`.
    pub fn delete_file(&self, path: impl AsRef<Path>) -> io::Result<()> {
        self.ambient_dir
            .remove_file(&path)
            .or_else(|_| self.ambient_dir.remove_dir_all(&path))
    }

    /// Creates a new directory at `path`.
    pub fn create_directory(&self, path: impl AsRef<Path>) -> io::Result<()> {
        self.ambient_dir.create_dir(path)
    }

    /// Creates an iterator for the contents of `path`.
    pub fn read_directory(&self, path: impl AsRef<Path>) -> io::Result<ReadDir> {
        self.ambient_dir.read_dir(path)
    }

    #[allow(unused)]
    /// Creates a new file at `path`.
    pub fn create_file(&self, path: impl AsRef<Path>) -> io::Result<()> {
        self.ambient_dir.create(path).map(|_| ())
    }

    // Drivers

    /// Accesses the given driver by its name.
    /// Currently this just iterates over all drivers, but in the future a
    /// `HashMap<&'static str, usize> may be used.
    fn get_driver_by_name(&mut self, name: &str) -> Option<(usize, &mut dyn Driver<T>)> {
        for (id, driver) in self.drivers.iter_mut().enumerate() {
            if driver.name() == name {
                return Some((id, driver.as_mut()));
            }
        }
        None
    }

    /// Loads each driver's functions.
    pub fn load_driver_functions(
        &mut self,
        linker: &mut ProcessLinker<T>,
        imported_modules: &[&str],
    ) -> wasmtime::Result<()> {
        for driver_name in imported_modules.iter().copied() {
            if let Some((id, driver)) = self.get_driver_by_name(driver_name) {
                driver.register_functions(linker, id)?;
            }
        }

        Ok(())
    }

    /// Gets the given driver by its id, casting it to its concrete type.
    ///
    /// # Panics
    ///
    /// Panics if the driver's id does not belong to this driver.
    pub fn get_driver<D: Any>(&mut self, id: usize) -> &mut D {
        self.drivers[id].as_any().downcast_mut().unwrap()
    }

    // Events

    /// Interns an event's name.
    /// Interned event names are used to make handler lookups faster.
    pub fn intern_event_name(&mut self, name: &str) -> SymbolU32 {
        self.interned_event_names.get_or_intern(name)
    }

    /// Sends the given event to the program.
    /// Event data is copied to the event itself, so programs can manipulate the address without
    /// affecting the event's data.
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
    /// Meant to be used by input drivers, where the root process can then call [resend_event] to
    /// pass input to its children.
    pub fn send_event_to_root(&mut self, event_name: &str, event_data: &[u8]) -> i32 {
        self.send_event(event_name, event_data, Id::default(), ROOT_PID)
    }

    /// Re-sends an event to the program.
    /// Event data is reference counted, meaning events can be efficiently passed around to other programs.
    pub fn resend_event(&mut self, event: &Event, sender: Pid, receiver: Pid) -> i32 {
        if let Some(receiver_process) = self.get_process_mut(receiver) {
            let event = Event::from_resent(event, sender);
            receiver_process.store.data_mut().push_event(event);
            0
        } else {
            -1
        }
    }

    /// Sets the currently handled event.
    pub fn set_current_event(&mut self, event_ptr: *mut Event) {
        self.current_event = NonNull::new(event_ptr);
    }

    /// Unsets the currently handled event.
    pub fn end_current_event(&mut self) {
        self.current_event = None;
    }

    /// Gets the currently handled event.
    /// This is used to allow programs to access the event's data.
    pub fn get_current_event(&self) -> &Event {
        unsafe {
            self.current_event
                .expect("Cannot get event data when outside of event handler or when handler is called without an event.")
                .as_ref()
        }
    }

    /// Gets the currently handled event's name.
    pub fn get_event_name(&self, interned_name: SymbolU32) -> &str {
        self.interned_event_names
            .resolve(interned_name)
            .unwrap_or("NO_EVENT_NAME")
    }

    // Proceses

    /// Sets the lookup name of the process `Pid`.
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
        let kernel: Kernel<()> = Kernel::new(root_dir.as_ref(), vec![]);

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
        let kernel: Kernel<()> = Kernel::new(root_dir.as_ref(), vec![]);

        kernel.create_file("foo").unwrap();
        kernel.copy_file("foo", "bar").unwrap();
        assert!(kernel.is_file("bar"));
        assert!(kernel.is_file("foo"));
        kernel.delete_file("foo").unwrap();
        kernel.delete_file("bar").unwrap();

        Ok(())
    }
}
