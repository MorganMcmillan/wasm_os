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
use raylib::RaylibHandle;
use raylib::ffi::KeyboardKey;
use string_interner::StringInterner;
use string_interner::backend::StringBackend;
use string_interner::symbol::SymbolU32;
use tokio::io::stderr;
use tokio::io::stdin;
use tokio::io::stdout;
use tokio::task;
use wasmtime::Caller;
use wasmtime::Config;
use wasmtime::Engine;

use crate::KERNEL;
use crate::async_file::AsyncFile;
use crate::driver::Driver;
use crate::event::Event;
use crate::id::Id;
use crate::id::IdStore;
use crate::process::Process;
use crate::wasm_process::WasmProcess;

pub type Pid = Id;
pub type ProcessLinker = wasmtime::Linker<Process>;
pub type ProcessContext<'a> = Caller<'a, Process>;

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
    // TODO: somehow create a default boot.wasm
    let _ = root_dir.create_dir("bios");
    let _ = root_dir.create_dir("rom");
    let _ = root_dir.create_dir("lib");
}

impl Kernel {
    pub fn new(root_dir: &Path, mut drivers: Vec<Box<dyn Driver>>) -> Self {
        let mut config = Config::new();
        config.strategy(wasmtime::Strategy::Cranelift);
        config.epoch_interruption(true);

        for (id, driver) in drivers.iter_mut().enumerate() {
            driver.accept_id(id);
        }

        let engine = Engine::new(&config).unwrap();
        let engine_clone = engine.clone();

        // Periodically interupt process execution
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            engine_clone.increment_epoch();
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
        !self.processes.id_is_valid(Id::new(1))
    }

    pub async fn run_boot(&mut self) {
        let kptr = self as *mut Self;
        unsafe {
            let root_pid = self.create_boot_process().await;
            let root = (*kptr).get_process_mut(root_pid).unwrap();
            let join_handle = task::spawn(root.run());

            self.get_process_mut(root_pid)
                .unwrap()
                .store
                .data_mut()
                .set_join_handle(join_handle);
        }
    }

    async fn create_boot_process(&mut self) -> Pid {
        async fn create_boot_process(
            kernel: &mut Kernel,
            path: &str,
        ) -> Result<Pid, CreateProcessError> {
            kernel
                .create_process(
                    path,
                    Pid::default(),
                    AsyncFile::stdin(),
                    AsyncFile::stdout(),
                    AsyncFile::stderr(),
                )
                .await
        }

        let root_process = match create_boot_process(self, USER_BOOT_PROCESS).await {
            Err(CreateProcessError::FileNotFound) => {
                match create_boot_process(self, ROM_BOOT_PROCESS).await {
                    Err(CreateProcessError::FileNotFound) => {
                        create_boot_process(self, BIOS_BOOT_PROCESS).await
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
        &mut self,
        path: &str,
        parent: Pid,
        stdin: AsyncFile,
        stdout: AsyncFile,
        stderr: AsyncFile,
    ) -> Result<Pid, CreateProcessError> {
        let pid = self
            .create_process(path, parent, stdin, stdout, stderr)
            .await?;
        let process = unsafe { KERNEL.get_process_mut(pid).unwrap() };
        let join_handle = task::spawn(process.run());

        self.get_process_mut(pid)
            .unwrap()
            .store
            .data_mut()
            .set_join_handle(join_handle);

        println!("Spawned process '{path}'");

        Ok(pid)
    }

    // Files

    pub fn directory_exists(&self, path: impl AsRef<Path>) -> bool {
        self.ambient_dir.is_dir(path)
    }

    #[allow(unused)]
    pub fn file_exists(&self, path: impl AsRef<Path>) -> bool {
        self.ambient_dir.is_file(path)
    }

    #[allow(unused)]
    pub fn fs_object_exists(&self, path: impl AsRef<Path>) -> bool {
        self.ambient_dir.exists(path)
    }

    pub fn get_absolute_path(&self, path: impl AsRef<Path>) -> io::Result<PathBuf> {
        self.ambient_dir.canonicalize(path)
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

    pub async fn create_process(
        &mut self,
        path: &str,
        parent: Pid,
        stdin: AsyncFile,
        stdout: AsyncFile,
        stderr: AsyncFile,
    ) -> Result<Pid, CreateProcessError> {
        if !path.ends_with(".wasm") {
            return Err(CreateProcessError::IncorrectFileType);
        }

        let binary = self.read_file(path)?;

        // TODO: figure out better way to handle process ids
        let path = std::path::Path::new(path);
        let label = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("_UNKNOWN_PROGRAM");

        let cwd = if parent.number() == 0 {
            PathBuf::new()
        } else {
            self.get_process(parent)
                .unwrap()
                .store
                .data()
                .current_working_directory
                .clone()
        };

        let mut process = Process::new(parent, label, cwd, stdin, stdout, stderr);

        for driver in self.drivers.iter_mut() {
            if let Some(process_state) = driver.create_process_state() {
                process.add_driver_state(driver.get_id(), process_state);
            }
        }

        let wasm_process = match WasmProcess::new(binary, &self.engine, process).await {
            Ok(w) => w,
            Err(e) => {
                eprintln!("Wasm error: {:?}", e);
                return Err(CreateProcessError::InvalidWasm);
            }
        };

        let pid = self.processes.new_id(wasm_process);

        self.get_process_mut(pid)
            .unwrap()
            .store
            .data_mut()
            .set_pid(pid);

        // If not root process:
        if parent.number() != 0 {
            self.get_process_mut(parent)
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

    pub fn update(&mut self, rl: &mut RaylibHandle, thread: &raylib::RaylibThread) {
        if rl.is_key_pressed(KeyboardKey::KEY_F11) {
            rl.toggle_fullscreen();
        }

        for driver in self.drivers.iter_mut() {
            driver.update(rl, thread);
        }
    }

    // Drivers

    pub fn get_driver_by_name(&mut self, name: &str) -> Option<&mut dyn Driver> {
        for driver in self.drivers.iter_mut() {
            if driver.name() == name {
                return Some(driver.as_mut());
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
            if let Some(driver) = self.get_driver_by_name(driver_name) {
                driver.register_functions(linker)?;
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
