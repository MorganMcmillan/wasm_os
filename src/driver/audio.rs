mod sample_queue;

use crate::{
    driver::Driver,
    kernel::{Kernel, ProcessContext, ProcessLinker},
    mut_cell::MutCell,
    system_functions,
};
use rodio::{MixerDeviceSink, Player};
use sample_queue::SampleQueue;
use std::any::Any;

pub struct AudioState {
    handle: MixerDeviceSink,
}

impl AudioState {
    pub fn new() -> Self {
        let handle = rodio::DeviceSinkBuilder::open_default_sink().unwrap();

        Self { handle }
    }
}

impl Driver for AudioState {
    fn name(&self) -> &'static str {
        "driver_audio"
    }

    fn update(
        &mut self,
        _kernel: &'static MutCell<Kernel>,
        _rl: &mut raylib::RaylibHandle,
        _thread: &raylib::RaylibThread,
    ) {
    }

    fn create_process_state(&mut self) -> Option<Box<dyn Any + Send>> {
        Some(Box::new(ProcessAudioState::new(Player::connect_new(
            self.handle.mixer(),
        ))))
    }

    fn register_functions(&self, linker: &mut ProcessLinker, id: usize) -> wasmtime::Result<()> {
        let name = self.name();

        linker.func_wrap(
            name,
            "play_sound",
            move |mut ctx: ProcessContext,
                  sound_ptr: i32,
                  sound_len: i32,
                  left_volume: i32,
                  right_volume: i32| {
                let left_volume = left_volume.min(255) as u8;
                let right_volume = right_volume.min(255) as u8;

                let sound = match system_functions::get_memory(&ctx, sound_ptr, sound_len) {
                    Ok(sound) => sound,
                    Err(e) => return e,
                };

                ctx.data_mut()
                    .get_driver_state_mut::<ProcessAudioState>(id)
                    .unwrap()
                    .play(sound, left_volume, right_volume);
                0
            },
        )?;

        Ok(())
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

#[allow(unused)]
pub struct ProcessAudioState {
    player: Player,
    sample_buffer: MutCell<SampleQueue>,
}

impl ProcessAudioState {
    pub fn new(player: Player) -> Self {
        let sample_buffer = MutCell::new(SampleQueue::new());
        player.append(sample_buffer.borrow_static());
        Self {
            player,
            sample_buffer,
        }
    }

    pub fn play(&mut self, samples: &[u8], left_volume: u8, right_volue: u8) -> usize {
        for (i, &sample) in samples.iter().enumerate() {
            let (left, right) = split_sample(sample, left_volume, right_volue);
            if !self.sample_buffer.borrow_static().push_sample(left, right) {
                return i;
            }
        }
        samples.len()
    }
}

fn split_sample(sample: u8, left_volume: u8, right_volue: u8) -> (u8, u8) {
    fn scale_sample(sample: u8, volume: u8) -> u8 {
        ((sample as u32 * volume as u32) / 255) as u8
    }
    (
        scale_sample(sample, left_volume),
        scale_sample(sample, right_volue),
    )
}
