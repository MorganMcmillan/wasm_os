use std::any::Any;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::io::ErrorKind::NotFound;
use std::io::Read;
use std::path::Path;
use std::ptr::NonNull;
use std::time::Duration;

use cap_std::ambient_authority;
use raylib::RaylibHandle;
use raylib::ffi::KeyboardKey;
use string_interner::StringInterner;
use string_interner::backend::StringBackend;
use string_interner::symbol::SymbolU32;
use tokio::task;
use wasmtime::Config;
use wasmtime::Engine;
use wasmtime::StoreContextMut;
use wasmtime::component::LinkerInstance;
use wasmtime_wasi::WasiCtx;

use crate::KERNEL;
use crate::driver::Driver;
use crate::event::Event;
use crate::event::EventData;
use crate::process::Process;
use crate::wasm_process::WasmProcess;

pub type Pid = u16;
pub type ProcessLinker<'a> = LinkerInstance<'a, Process>;
pub type ProcessContext<'a> = StoreContextMut<'a, Process>;

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
    pub processes: Vec<Option<WasmProcess>>,
    // A map of process names to Pids
    process_names: HashMap<Box<str>, Pid>,
    current_event: Option<NonNull<Event>>,
    interned_event_names: StringInterner<StringBackend>,
    // The top-level directory for which programs can be executed from
    ambient_dir: cap_std::fs::Dir,
    root_dir_path: Box<str>,
}

unsafe impl Send for Kernel {}
unsafe impl Sync for Kernel {}

const BIOS_BOOT_PROCESS: &str = "bios/boot.wasm";
const ROM_BOOT_PROCESS: &str = "rom/boot.wasm";
const USER_BOOT_PROCESS: &str = "boot.wasm";

impl Kernel {
    pub fn new(root_dir: &str, mut drivers: Vec<Box<dyn Driver>>) -> Self {
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

        Self {
            engine,
            drivers,
            processes: Vec::new(),
            process_names: HashMap::new(),
            current_event: None,
            interned_event_names: StringInterner::new(),
            ambient_dir: cap_std::fs::Dir::open_ambient_dir(
                Path::new(root_dir),
                ambient_authority(),
            )
            .unwrap(),
            root_dir_path: root_dir.into(),
        }
    }

    pub fn root_exited(&self) -> bool {
        !matches!(self.processes.first(), Some(Some(_)))
    }

    // Loads each driver's functions.
    pub fn load_driver_functions(&mut self, linker: &mut ProcessLinker) -> wasmtime::Result<()> {
        for driver in self.drivers.iter_mut() {
            driver.register_functions(linker)?;
        }

        Ok(())
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
        let root_process = match self.create_process(USER_BOOT_PROCESS, 0).await {
            Err(CreateProcessError::FileNotFound) => {
                match self.create_process(ROM_BOOT_PROCESS, 0).await {
                    Err(CreateProcessError::FileNotFound) => {
                        self.create_process(BIOS_BOOT_PROCESS, 0).await
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
    ) -> Result<Pid, CreateProcessError> {
        let pid = self.create_process(path, parent).await?;
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

    fn read_file(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, CreateProcessError> {
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

    fn get_root_dir(&self) -> &str {
        &self.root_dir_path
    }

    pub async fn create_process(
        &mut self,
        path: &str,
        parent: Pid,
    ) -> Result<Pid, CreateProcessError> {
        if !path.ends_with(".wasm") {
            return Err(CreateProcessError::IncorrectFileType);
        }

        let binary = self.read_file(path)?;

        let mut builder = WasiCtx::builder();
        builder.initial_cwd(self.get_root_dir());
        let wasi_ctx = builder.build();

        // TODO: figure out better way to handle process ids
        let pid = self.processes.len() as u16 + 1;
        let path = std::path::Path::new(path);
        let label = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("_UNKNOWN_PROGRAM");

        let mut process = Process::new(wasi_ctx, pid, parent, label);

        for driver in self.drivers.iter_mut() {
            if let Some(process_state) = driver.create_process_state() {
                process.add_driver_state(driver.get_id(), process_state);
            }
        }

        // TODO: add ability to set kernel filesystem root
        let wasm_state = match WasmProcess::new(binary, &self.engine, process).await {
            Ok(w) => w,
            Err(e) => {
                eprintln!("Wasm error: {:?}", e);
                return Err(CreateProcessError::InvalidWasm);
            }
        };

        self.processes.push(Some(wasm_state));

        // If not root process:
        if parent != 0 {
            self.get_process_mut(parent)
                .expect("Expected a parent process to exist.")
                .store
                .data_mut()
                .add_child(pid);
        }

        Ok(pid)
    }

    pub fn get_process(&self, pid: Pid) -> Option<&WasmProcess> {
        match self.processes.get((pid - 1) as usize) {
            Some(Some(process)) => Some(process),
            _ => None,
        }
    }

    pub fn get_process_mut(&mut self, pid: Pid) -> Option<&mut WasmProcess> {
        match self.processes.get_mut((pid - 1) as usize) {
            Some(Some(process)) => Some(process),
            _ => None,
        }
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

    pub fn get_driver<T: Any>(&mut self, id: usize) -> &mut T {
        self.drivers[id].as_any().downcast_mut().unwrap()
    }

    // Events

    pub fn intern_event_name(&mut self, name: &str) -> SymbolU32 {
        self.interned_event_names.get_or_intern(name)
    }

    pub fn send_event(&mut self, event_name: &str, event_data: &[u8], sender: Pid, receiver: Pid) {
        if event_data.len() > size_of::<EventData>() {
            // TODO: replace this panic with an error code or something
            panic!(
                "Event data cannot be larger than 512 bytes, instead got {} bytes",
                event_data.len()
            );
        }
        let interned_name = self.interned_event_names.get_or_intern(event_name);
        let mut copied_data = [0u8; size_of::<EventData>()];
        for (i, &byte) in event_data.iter().enumerate() {
            copied_data[i] = byte;
        }

        let event = Event::new(copied_data, event_data.len() as u16, sender, interned_name);

        let receiver_process = self.get_process_mut(receiver).unwrap();
        receiver_process.store.data_mut().push_event(event);
    }

    pub fn set_current_event(&mut self, event_ptr: *mut Event) {
        self.current_event = NonNull::new(event_ptr);
    }

    pub fn get_current_event(&self) -> &Event {
        unsafe {
            self.current_event
                .expect("Attempted to call an event handler without any event data.")
                .as_ref()
        }
    }

    pub fn get_event_name(&self, interned_name: SymbolU32) -> &str {
        self.interned_event_names
            .resolve(interned_name)
            .unwrap_or("NO_EVENT_NAME")
    }

    pub fn set_process_name(&mut self, pid: u16, name: &str) -> bool {
        match self.process_names.entry(name.into()) {
            Entry::Vacant(entry) => {
                entry.insert(pid);
                true
            }
            Entry::Occupied(_) => false,
        }
    }

    pub fn get_pid_by_name(&self, name: &str) -> Pid {
        self.process_names.get(name).copied().unwrap_or(0)
    }
}
