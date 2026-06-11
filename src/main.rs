use raylib::{ffi::Rectangle, prelude::*};
use wasmtime::*;

const FRAMEBUFFER_WIDTH: usize = 384;
const FRAMEBUFFER_HEIGHT: usize = 216;
const FRAMEBUFFER_SIZE: usize = FRAMEBUFFER_WIDTH * FRAMEBUFFER_HEIGHT;

fn get_memory_slice<'a>(instance: &'a Instance, store: &'a mut Store<Kernel>) -> &'a [u8] {
    let framebuffer = instance.get_memory(&mut *store, "memory").unwrap();
    framebuffer.data(store)
}

fn load_system_functions(linker: &mut Linker<Kernel>) -> wasmtime::Result<()> {
    // Load "set_active_framebuffer"
    linker.func_wrap(
        "env",
        "set_active_framebuffer",
        |mut caller: Caller<Kernel>, framebuffer: i32| {
            caller.data_mut().framebuffer_address = Some(framebuffer as u32);
        },
    )?;

    Ok(())
}

/// Takes a byte in the format `0bRRRGGGBB` and returns an RGB tuple, normalized to `[0, 255]`.
fn byte_to_rgb(byte: u8) -> (u8, u8, u8) {
    let r: u8 = (byte & 0b11100000) >> 5;
    let g: u8 = (byte & 0b00011100) >> 2;
    let b: u8 = byte & 0b00000011;
    (r.saturating_mul(37), g.saturating_mul(37), b * 85)
}

fn draw_framebuffer(framebuffer: &[u8], texture: &mut Texture2D) {
    let mut fb_rgba8888 = [0u8; (FRAMEBUFFER_WIDTH * FRAMEBUFFER_HEIGHT * 4) as usize];
    for (i, pixel) in framebuffer.iter().enumerate() {
        let (r, g, b) = byte_to_rgb(*pixel);
        fb_rgba8888[i * 4] = r;
        fb_rgba8888[i * 4 + 1] = g;
        fb_rgba8888[i * 4 + 2] = b;
        fb_rgba8888[i * 4 + 3] = 255;
    }
    texture.update_texture(&fb_rgba8888).unwrap();
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

    fn run_init(&mut self) -> wasmtime::Result<()> {
        let init_func = self
            .instance
            .get_typed_func::<(), ()>(&mut self.store, "init")?;
        init_func.call(&mut self.store, ())?;
        Ok(())
    }

    fn update(&mut self) {
        // Call update function
        let update_func = self
            .instance
            .get_typed_func::<(), ()>(&mut self.store, "update")
            .unwrap();
        update_func.call(&mut self.store, ()).unwrap();

        // Draw framebuffer
        let kernel: *mut Kernel = self.store.data_mut();
        if let Some(address) = unsafe { (*kernel).framebuffer_address } {
            let memory = get_memory_slice(&self.instance, &mut self.store);
            unsafe {
                draw_framebuffer(
                    &memory[address as usize..(address as usize + FRAMEBUFFER_SIZE)],
                    &mut (*kernel).framebuffer_texture,
                );
            }
        }
    }
}

struct Kernel {
    framebuffer_address: Option<u32>,
    framebuffer_texture: Texture2D,
}

impl Kernel {
    fn new(framebuffer_texture: Texture2D) -> Self {
        Self {
            framebuffer_address: None,
            framebuffer_texture,
        }
    }
}

fn main() -> wasmtime::Result<()> {
    let (mut rl, thread) = raylib::init()
        .size(FRAMEBUFFER_WIDTH as i32 * 2, FRAMEBUFFER_HEIGHT as i32 * 2)
        .title("WasmOS Test")
        .resizable()
        .build();

    rl.set_target_fps(20);

    let img = unsafe {
        raylib::ffi::GenImageColor(
            FRAMEBUFFER_WIDTH as i32,
            FRAMEBUFFER_HEIGHT as i32,
            Color::BLACK,
        )
    };
    let img = unsafe { Image::from_raw(img) };
    let texture = rl.load_texture_from_image(&thread, &img).unwrap();

    let engine = Engine::default();
    let mut wasmstate = WasmState::new(&engine, Kernel::new(texture))?;

    wasmstate.run_init().unwrap();

    while !rl.window_should_close() {
        wasmstate.update();

        if rl.is_key_pressed(KeyboardKey::KEY_F11) {
            rl.toggle_fullscreen();
        }

        let screen_width = rl.get_screen_width();
        let screen_height = rl.get_screen_height();

        let mut d = rl.begin_drawing(&thread);
        d.draw_texture_pro(
            &wasmstate.store.data().framebuffer_texture,
            Rectangle::new(
                0.0,
                0.0,
                FRAMEBUFFER_WIDTH as f32,
                FRAMEBUFFER_HEIGHT as f32,
            ),
            Rectangle::new(0.0, 0.0, screen_width as f32, screen_height as f32),
            Vector2::new(0.0, 0.0),
            0.0,
            Color::WHITE,
        );
    }

    Ok(())
}
