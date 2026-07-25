use raylib::RaylibHandle;
use wasmtime::Linker;

use crate::process::Process;

pub mod draw;
pub mod input;

pub trait Driver {
    fn register_functions(&self, linker: &mut Linker<Process>) -> wasmtime::Result<()>;
    fn update(&mut self, rl: &mut RaylibHandle);
}
