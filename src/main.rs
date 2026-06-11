use raylib::prelude::*;
use wasmtime::{Strategy::Cranelift, *};

use crate::draw::DrawState;

mod draw;
mod input;

fn get_memory_slice<'a>(instance: &'a Instance, store: &'a mut Store<Kernel>) -> &'a [u8] {
    let memory = instance.get_memory(&mut *store, "memory").unwrap();
    memory.data(store)
}

fn get_framebuffer<'a>(instance: &'a Instance, store: &'a mut Store<Kernel>, address: usize) -> &'a [u8] {
    let memory = get_memory_slice(instance, store);
    &memory[address..(address + draw::FRAMEBUFFER_SIZE)]
}

fn load_system_functions(linker: &mut Linker<Kernel>) -> wasmtime::Result<()> {
    // Load "set_active_framebuffer"
    linker.func_wrap(
        "env",
        "set_active_framebuffer",
        |mut caller: Caller<Kernel>, framebuffer: i32| {
            caller.data_mut().drawstate.framebuffer_address = Some(framebuffer as u32);
        },
    )?;

    linker.func_wrap(
        "env",
        "get_mouse_x",
        |caller: Caller<Kernel>| {
            caller.data().mousestate.x as i32
        }
    )?;

    linker.func_wrap(
        "env",
        "get_mouse_y",
        |caller: Caller<Kernel>| {
            caller.data().mousestate.y as i32
        }
    )?;

    Ok(())
}

struct WasmState {
    instance: Instance,
    store: Store<Kernel>,
}

impl WasmState {
    fn new(engine: &Engine, kernel: Kernel) -> wasmtime::Result<Self> {
        let binary = std::fs::read("test.wasm").unwrap();
        // Modules are compiled from text or binary
        let module = Module::new(&engine, binary)?;

        // Linkers expose host functions
        let mut linker = Linker::new(&engine);

        load_system_functions(&mut linker)?;

        // All wasm objects operate in the context of a store.
        // A store is used to store host-specific data of a given type.
        let mut store = Store::new(&engine, kernel);

        Ok(Self {
            instance: linker.instantiate(&mut store, &module)?,
            store,
        })
    }

    /// Calls the `init` function of the program
    fn init(&mut self) -> wasmtime::Result<()> {
        let init_func = self
            .instance
            .get_typed_func::<(), ()>(&mut self.store, "init")?;
        init_func.call(&mut self.store, ())?;
        Ok(())
    }

    /// Calls the `update` function of the program
    fn update(&mut self) {
        let update_func = self
            .instance
            .get_typed_func::<(), ()>(&mut self.store, "update")
            .unwrap();
        update_func.call(&mut self.store, ()).unwrap();
    }

    /// Draws the program's framebuffer to a texture
    fn upload_framebuffer(&mut self) {
        unsafe {
            let kernel: *mut Kernel = self.store.data_mut();
            if let Some(address) = (*kernel).drawstate.framebuffer_address {
                let framebuffer = get_framebuffer(&self.instance, &mut self.store, address as usize);
                (*kernel).drawstate.upload_framebuffer(framebuffer);
            }
        }
    }
}

struct Kernel {
    drawstate: draw::DrawState,
    mousestate: input::MouseState,
}

impl Kernel {
    fn new(drawstate: draw::DrawState) -> Self {
        Self {
            drawstate,
            mousestate: input::MouseState::new(),
        }
    }
}

fn main() -> wasmtime::Result<()> {
    let (mut rl, thread) = raylib::init()
        .size(draw::FRAMEBUFFER_WIDTH as i32 * 2, draw::FRAMEBUFFER_HEIGHT as i32 * 2)
        .title("WasmOS Test")
        .resizable()
        .build();

    rl.set_target_fps(20);

    let img = unsafe {
        raylib::ffi::GenImageColor(
            draw::FRAMEBUFFER_WIDTH as i32,
            draw::FRAMEBUFFER_HEIGHT as i32,
            Color::BLACK,
        )
    };
    let img = unsafe { Image::from_raw(img) };
    let texture = rl.load_texture_from_image(&thread, &img).unwrap();

    let mut config = Config::new();
    config.strategy(Cranelift);
    let engine = Engine::new(&config)?;
    let drawstate = DrawState::new(texture);
    let kernel = Kernel::new(drawstate);
    let mut wasmstate = WasmState::new(&engine, kernel)?;

    wasmstate.init().unwrap();

    while !rl.window_should_close() {
        wasmstate.update();
        wasmstate.upload_framebuffer();

        if rl.is_key_pressed(KeyboardKey::KEY_F11) {
            rl.toggle_fullscreen();
        }

        let screen_width = rl.get_screen_width();
        let screen_height = rl.get_screen_height();

        let mx = rl.get_mouse_x();
        let my = rl.get_mouse_y();
        let width = rl.get_screen_width();
        let height = rl.get_screen_height();

        println!("mx: {mx}, my: {my}");
        println!("width: {width}, height: {height}");
        println!("mousestate: {:?}", wasmstate.store.data().mousestate);

        wasmstate.store.data_mut().mousestate.update(mx, my, width, height);

        let mut d = rl.begin_drawing(&thread);
        wasmstate.store.data().drawstate.draw_framebuffer(&mut d, screen_width, screen_height);
    }

    Ok(())
}
