use std::{any::Any, num::NonZeroU32};

use rodio::{MixerDeviceSink, Player, nz};

use crate::{
    driver::Driver,
    kernel::{ProcessContext, ProcessLinker},
    system_functions,
};

pub const SAMPLE_RATE: u32 = 44100;

// TODO: currently only single channel audio is supported. To support 2 channels, samples from both
// will have to be interleaved

pub struct PcmBuffer {
    samples: Box<[u8]>,
    pos: usize,
}

impl PcmBuffer {
    pub fn new(samples: &[u8]) -> Self {
        Self {
            samples: samples.to_owned().into(),
            pos: 0,
        }
    }
}

impl Iterator for PcmBuffer {
    type Item = rodio::Sample;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = *self.samples.get(self.pos)?;
        self.pos += 1;
        Some(sample as rodio::Sample / 255.0)
    }
}

impl rodio::Source for PcmBuffer {
    fn current_span_len(&self) -> Option<usize> {
        if self.pos >= self.samples.len() {
            Some(0)
        } else {
            Some(self.samples.len())
        }
    }

    fn channels(&self) -> rodio::ChannelCount {
        nz!(1)
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        NonZeroU32::new(SAMPLE_RATE).unwrap()
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

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

    fn update(&mut self, _rl: &mut raylib::RaylibHandle, _thread: &raylib::RaylibThread) {}

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
            move |mut ctx: ProcessContext, sound_ptr: i32, sound_len: i32| {
                let sound = match system_functions::get_memory(&ctx, sound_ptr, sound_len) {
                    Ok(sound) => sound,
                    Err(e) => return e,
                };

                let samples = PcmBuffer::new(sound);
                ctx.data_mut()
                    .get_driver_state_mut::<ProcessAudioState>(id)
                    .unwrap()
                    .play(samples);
                0
            },
        )?;

        Ok(())
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

pub struct ProcessAudioState {
    player: Player,
}

impl ProcessAudioState {
    pub fn new(player: Player) -> Self {
        Self { player }
    }

    pub fn play(&self, samples: PcmBuffer) {
        self.player.append(samples);
    }
}
