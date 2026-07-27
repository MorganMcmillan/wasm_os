use std::{any::Any, num::NonZeroU32};

use rodio::{MixerDeviceSink, Player, nz};
use wasmtime::component::WasmList;

use crate::{
    driver::Driver,
    kernel::{ProcessContext, ProcessLinker},
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
    driver_id: usize,
}

impl AudioState {
    pub fn new() -> Self {
        let handle = rodio::DeviceSinkBuilder::open_default_sink().unwrap();

        Self {
            handle,
            driver_id: 0,
        }
    }
}

impl Driver for AudioState {
    fn update(&mut self, _rl: &mut raylib::RaylibHandle, _thread: &raylib::RaylibThread) {}

    fn accept_id(&mut self, id: usize) {
        self.driver_id = id;
    }

    fn get_id(&self) -> usize {
        self.driver_id
    }

    fn create_process_state(&mut self) -> Option<Box<dyn Any + Send>> {
        Some(Box::new(ProcessAudioState::new(Player::connect_new(
            self.handle.mixer(),
        ))))
    }

    fn register_functions(&self, linker: &mut ProcessLinker) -> wasmtime::Result<()> {
        let id = self.driver_id;

        linker.func_wrap(
            "play-sound",
            move |mut ctx: ProcessContext, (sound,): (WasmList<u8>,)| {
                let samples = PcmBuffer::new(sound.as_le_slice(&ctx));
                ctx.data_mut()
                    .get_driver_state_mut::<ProcessAudioState>(id)
                    .unwrap()
                    .play(samples);
                Ok(())
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
