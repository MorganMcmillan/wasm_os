use raylib::RaylibHandle;
use raylib::RaylibThread;
use raylib::ffi::KeyboardKey;
use string_interner::StringInterner;
use string_interner::backend::StringBackend;
use wasmtime::Engine;

use crate::draw;
use crate::event::Event;
use crate::input;
use crate::process::Process;

pub type Pid = u16;

enum CreateProcessError {
    FileNotFound,
    InvalidWasm,
    IncorrectFileType,
}

pub struct Kernel {
    engine: Engine,
    pub drawstate: draw::DrawState,
    pub mousestate: input::MouseState,
    // A sparse map of Pids to processes
    pub processes: Vec<Option<Process>>,
    // The current pid of the running program
    current_pid: Pid,
    str_intern_state: StringInterner<StringBackend>,
}

const BIOS_BOOT_PROCESS: &str = "/bios/boot.wasm";
const ROM_BOOT_PROCESS: &str = "/rom/boot.wasm";
const USER_BOOT_PROCESS: &str = "/boot.wasm";

impl Kernel {
    pub fn new(engine: Engine, drawstate: draw::DrawState) -> Self {
        let mut kernel = Self {
            engine,
            drawstate,
            mousestate: input::MouseState::new(),
            processes: Vec::new(),
            current_pid: 0,
            str_intern_state: StringInterner::new(),
        };
        kernel.open_root_process();
        kernel
    }

    fn open_root_process(&mut self) {
        let is_ok = self.create_process(USER_BOOT_PROCESS, &[]).is_ok()
            || self.create_process(ROM_BOOT_PROCESS, &[]).is_ok()
            || self.create_process(BIOS_BOOT_PROCESS, &[]).is_ok();

        if !is_ok {
            panic!("Could not create boot process for any 'bios.wasm'.")
        }
    }

    pub fn root_exited(&self) -> bool {
        match self.processes.get(0) {
            Some(Some(_)) => false,
            _ => true,
        }
    }

    pub fn create_process(&mut self, path: &str, args: &[&str]) -> Result<Pid, CreateProcessError> {
        todo!("unimplemented")
    }

    pub fn get_process(&mut self, pid: Pid) -> Option<&mut Process> {
        match self.processes.get_mut((pid - 1) as usize) {
            Some(Some(process)) => Some(process),
            _ => None,
        }
    }

    pub fn get_current_pid(&self) -> Pid {
        self.current_pid
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

    pub fn send_event(&mut self, event_name: &str, event_data: &[u8], sender: Pid, receiver: Pid) {
        if event_data.len() > 512 {
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

        let receiver_process = self.get_process(receiver).unwrap();
        receiver_process.push_event(event);
    }
}
