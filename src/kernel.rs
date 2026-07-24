use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::io::ErrorKind::NotFound;
use std::ptr::NonNull;

use raylib::RaylibHandle;
use raylib::ffi::KeyboardKey;
use string_interner::StringInterner;
use string_interner::backend::StringBackend;
use string_interner::symbol::SymbolU32;
use tokio::task;
use wasmtime::Engine;

use crate::KERNEL;
use crate::draw;
use crate::event::Event;
use crate::event::EventData;
use crate::input;
use crate::process::Process;
use crate::wasm_process::WasmProcess;

pub type Pid = u16;

#[derive(Debug)]
pub enum CreateProcessError {
    FileNotFound,
    InvalidWasm,
    IncorrectFileType,
    Other,
}

pub struct Kernel {
    engine: Engine,
    pub drawstate: draw::DrawState,
    pub mousestate: input::MouseState,
    // A sparse map of Pids to processes
    pub processes: Vec<Option<WasmProcess>>,
    // A map of process names to Pids
    process_names: HashMap<Box<str>, Pid>,
    // The current pid of the running program
    current_pid: Pid,
    current_event: Option<NonNull<Event>>,
    str_intern_state: StringInterner<StringBackend>,
}

unsafe impl Send for Kernel {}
unsafe impl Sync for Kernel {}

const BIOS_BOOT_PROCESS: &str = "bios/boot.wasm";
const ROM_BOOT_PROCESS: &str = "rom/boot.wasm";
const USER_BOOT_PROCESS: &str = "boot.wasm";

impl Kernel {
    pub fn new(engine: Engine, drawstate: draw::DrawState) -> Self {
        Self {
            engine,
            drawstate,
            mousestate: input::MouseState::new(),
            processes: Vec::new(),
            process_names: HashMap::new(),
            current_pid: 0,
            current_event: None,
            str_intern_state: StringInterner::new(),
        }
    }

    pub fn root_exited(&self) -> bool {
        !matches!(self.processes.first(), Some(Some(_)))
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

        Ok(pid)
    }

    pub async fn create_process(
        &mut self,
        path: &str,
        parent: Pid,
    ) -> Result<Pid, CreateProcessError> {
        if !path.ends_with(".wasm") {
            return Err(CreateProcessError::IncorrectFileType);
        }

        // TODO: prepend root directory to path
        let binary = match std::fs::read(path) {
            Ok(bin) => bin,
            Err(e) => match e.kind() {
                NotFound => return Err(CreateProcessError::FileNotFound),
                _ => return Err(CreateProcessError::Other),
            },
        };

        // TODO: figure out better way to handle process ids
        let pid = self.processes.len() as u16 + 1;
        let path = std::path::Path::new(path);
        let label = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("_UNKNOWN_PROGRAM");
        let process = Process::new(pid, parent, label);

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

    pub fn set_current_pid(&mut self, pid: Pid) {
        self.current_pid = pid;
    }

    #[allow(dead_code)]
    pub fn get_current_pid(&self) -> Pid {
        self.current_pid
    }

    #[allow(dead_code)]
    pub fn get_current_process(&self) -> &WasmProcess {
        self.get_process(self.current_pid).unwrap()
    }

    #[allow(dead_code)]
    pub fn get_current_process_mut(&mut self) -> &mut WasmProcess {
        self.get_process_mut(self.current_pid).unwrap()
    }

    pub fn update(&mut self, rl: &mut RaylibHandle) {
        if rl.is_key_pressed(KeyboardKey::KEY_F11) {
            rl.toggle_fullscreen();
        }

        self.mousestate.update(rl);
    }

    pub fn upload_framebuffer(&mut self) {
        let kernel = self as *mut Self;
        unsafe {
            if let Some((pid, address)) = (*kernel).drawstate.framebuffer_address {
                let process = (*kernel).get_process(pid).unwrap();
                let framebuffer = process.get_memory(address as usize, draw::FRAMEBUFFER_SIZE);
                (*kernel).drawstate.upload_framebuffer(framebuffer);
            } else {
                eprintln!("Warning: no framebuffer set!");
            }
        }
    }

    // Events

    pub fn intern_event_name(&mut self, name: &str) -> SymbolU32 {
        self.str_intern_state.get_or_intern(name)
    }

    pub fn send_event(&mut self, event_name: &str, event_data: &[u8], sender: Pid, receiver: Pid) {
        if event_data.len() > size_of::<EventData>() {
            // TODO: replace this panic with an error code or something
            panic!(
                "Event data cannot be larger than 512 bytes, instead got {} bytes",
                event_data.len()
            );
        }
        let interned_name = self.str_intern_state.get_or_intern(event_name);
        let mut copied_data = [0u8; size_of::<EventData>()];
        for (i, &byte) in event_data.iter().enumerate() {
            copied_data[i] = byte;
        }

        let event = Event::new(copied_data, sender, interned_name);

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
        self.str_intern_state
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
