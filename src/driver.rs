use std::any::Any;

use raylib::{RaylibHandle, RaylibThread};

pub type RaylibUserdata = (&'static mut RaylibHandle, &'static RaylibThread);

use crate::{
    kernel::{Kernel, ProcessLinker},
    mut_cell::MutCell,
};

pub mod audio;
pub mod input;
pub mod screen;

pub trait Driver<T: 'static>: Any {
    // This driver's name.
    // IMPORTANT: this name MUST start with "driver_"
    fn name(&self) -> &'static str;

    /// Regester wrapped functions for a new process
    /// The driver id is needed to access the driver instance of the kernel, and per-process state
    fn register_functions(&self, linker: &mut ProcessLinker<T>, id: usize) -> wasmtime::Result<()>;

    /// Update this driver's state whenever the kernel's update executes.
    fn update(&mut self, kernel: &'static MutCell<Kernel<T>>, userdata: &mut T);

    // Creates an optional per-process state.
    fn create_process_state(&mut self) -> Option<Box<dyn Any + Send>> {
        None
    }

    // Used to access the concrete type of this driver.
    fn as_any(&mut self) -> &mut dyn Any;
}
