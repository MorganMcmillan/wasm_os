use raylib::RaylibHandle;
use wasmtime::component::LinkerInstance;

use crate::process::Process;

pub mod audio;
pub mod draw;
pub mod input;

pub trait Driver: std::any::Any {
    // Regester wrapped functions for a new process
    fn register_functions(&self, linker: &mut LinkerInstance<Process>) -> wasmtime::Result<()>;

    // Update this driver's state whenever the kernel's update executes.
    fn update(&mut self, rl: &mut RaylibHandle, thread: &raylib::RaylibThread);

    // Has the driver accept the given id.
    // Needed to access per-process driver states.
    // May be ignored if no process state is created.
    fn accept_id(&mut self, id: usize);

    fn create_process_state(&mut self) -> Option<Box<dyn std::any::Any>> {
        None
    }
}
