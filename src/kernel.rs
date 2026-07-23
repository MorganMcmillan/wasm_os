use std::io::ErrorKind::NotFound;

use raylib::RaylibHandle;
use raylib::RaylibThread;
use raylib::ffi::KeyboardKey;
use string_interner::StringInterner;
use string_interner::backend::StringBackend;
use string_interner::symbol::SymbolU32;
use tokio::task;
use wasmtime::Engine;

use crate::draw;
use crate::event::Event;
use crate::event::EventData;
use crate::input;
use crate::process::Process;
use crate::wasm_state::WasmState;

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
    pub processes: Vec<Option<Process>>,
    // The current pid of the running program
    current_pid: Pid,
    current_event: Option<*const Event>,
    str_intern_state: StringInterner<StringBackend>,
}

const BIOS_BOOT_PROCESS: &str = "bios/boot.wasm";
const ROM_BOOT_PROCESS: &str = "rom/boot.wasm";
const USER_BOOT_PROCESS: &str = "boot.wasm";

impl Kernel {
    pub async fn new(engine: Engine, drawstate: draw::DrawState) -> Self {
        let mut kernel = Self {
            engine,
            drawstate,
            mousestate: input::MouseState::new(),
            processes: Vec::new(),
            current_pid: 0,
            current_event: None,
            str_intern_state: StringInterner::new(),
        };

        let kernel_ptr = &mut kernel as *mut Kernel;
        unsafe {
            let root_pid = (*kernel_ptr).create_root_process().await;
            let root = (*kernel_ptr).get_process_mut(root_pid).unwrap();
            let join_handle = task::spawn(root.run());

            (*kernel_ptr)
                .get_process_mut(root_pid)
                .unwrap()
                .set_join_handle(join_handle);
        }

        kernel
    }

    async fn create_root_process(&mut self) -> Pid {
        let root_process = match self.create_process(USER_BOOT_PROCESS).await {
            Err(CreateProcessError::FileNotFound) => {
                match self.create_process(ROM_BOOT_PROCESS).await {
                    Err(CreateProcessError::FileNotFound) => {
                        self.create_process(BIOS_BOOT_PROCESS).await
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

    pub fn root_exited(&self) -> bool {
        !matches!(self.processes.first(), Some(Some(_)))
    }

    pub async fn create_process(&mut self, path: &str) -> Result<Pid, CreateProcessError> {
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

        let self_ptr = self as *mut Kernel;

        // TODO: add ability to set kernel filesystem root
        let wasm_state = match WasmState::new(binary, &self.engine, self_ptr).await {
            Ok(w) => w,
            Err(e) => {
                eprintln!("Wasm error: {:?}", e);
                return Err(CreateProcessError::InvalidWasm);
            }
        };

        let process = Process::new(wasm_state);
        // TODO: figure out better way to handle process ids
        self.processes.push(Some(process));

        let pid = self.processes.len() as u16;
        Ok(pid)
    }

    pub fn get_process(&self, pid: Pid) -> Option<&Process> {
        match self.processes.get((pid - 1) as usize) {
            Some(Some(process)) => Some(process),
            _ => None,
        }
    }

    pub fn get_process_mut(&mut self, pid: Pid) -> Option<&mut Process> {
        match self.processes.get_mut((pid - 1) as usize) {
            Some(Some(process)) => Some(process),
            _ => None,
        }
    }

    pub fn get_current_pid(&self) -> Pid {
        self.current_pid
    }

    pub fn get_current_process(&self) -> &Process {
        self.get_process(self.current_pid).unwrap()
    }

    pub fn get_current_process_mut(&mut self) -> &mut Process {
        self.get_process_mut(self.current_pid).unwrap()
    }

    pub fn update(&mut self, rl: &mut RaylibHandle, thread: &RaylibThread) {
        if rl.is_key_pressed(KeyboardKey::KEY_F11) {
            rl.toggle_fullscreen();
        }

        let screen_width = rl.get_screen_width();
        let screen_height = rl.get_screen_height();
        let mx = rl.get_mouse_x();
        let my = rl.get_mouse_y();

        self.mousestate.update(mx, my, screen_width, screen_height);

        let mut d = rl.begin_drawing(thread);
        self.drawstate
            .draw_framebuffer(&mut d, screen_width, screen_height);
    }

    pub fn upload_framebuffer(&mut self) {
        let kernel = self as *mut Self;
        unsafe {
            if let Some((pid, address)) = (*kernel).drawstate.framebuffer_address {
                let process = (*kernel).get_process(pid).unwrap();
                let framebuffer = process.get_framebuffer(address as usize);
                (*kernel).drawstate.upload_framebuffer(framebuffer);
            }
        }
    }

    // Events
    //
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
        let mut copied_data = [0u8; 512];
        for (i, &byte) in event_data.iter().enumerate() {
            copied_data[i] = byte;
        }

        let event = Event::new(copied_data, sender, interned_name);

        let receiver_process = self.get_process_mut(receiver).unwrap();
        receiver_process.push_event(event);
    }

    pub fn set_current_event(&mut self, event_ptr: *const Event) {
        self.current_event = Some(event_ptr);
    }

    pub fn get_current_event(&self) -> &Event {
        unsafe {
            &*self
                .current_event
                .expect("Attempted to call an event handler without any event data.")
        }
    }

    pub fn get_event_name(&self, interned_name: SymbolU32) -> &str {
        self.str_intern_state
            .resolve(interned_name)
            .unwrap_or("NO_EVENT_NAME")
    }
}
