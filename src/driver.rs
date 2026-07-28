use std::any::Any;

use raylib::{RaylibHandle, RaylibThread};

use crate::kernel::ProcessLinker;

pub mod audio;
pub mod draw;
pub mod input;

pub trait Driver: Any {
    // This driver's name
    fn name(&self) -> &'static str;

    // Regester wrapped functions for a new process
    fn register_functions(&self, linker: &mut ProcessLinker) -> wasmtime::Result<()>;

    // Update this driver's state whenever the kernel's update executes.
    fn update(&mut self, rl: &mut RaylibHandle, thread: &RaylibThread);

    // Has the driver accept the given id.
    // Needed to access per-process driver states.
    // May be ignored if no process state is created.
    fn accept_id(&mut self, id: usize);

    fn get_id(&self) -> usize;

    // Creates an optional per-process state.
    fn create_process_state(&mut self) -> Option<Box<dyn Any + Send>> {
        None
    }

    // Used to access the concrete type of this driver.
    fn as_any(&mut self) -> &mut dyn Any;
}
