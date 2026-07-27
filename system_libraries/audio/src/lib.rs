#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn extern_play_sound(sound_ptr: *const u8, sound_len: i32) -> i32;
}

pub fn play_sound(sound: &[u8]) -> i32 {
    unsafe { extern_play_sound(sound.as_ptr(), sound.len() as i32) }
}
