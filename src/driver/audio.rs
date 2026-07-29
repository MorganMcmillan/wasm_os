use std::{
    any::Any,
    num::NonZeroU32,
    sync::{Arc, Mutex},
};

use rodio::{MixerDeviceSink, Player, nz};

use crate::{
    driver::Driver,
    kernel::{Kernel, ProcessContext, ProcessLinker},
    mut_cell::MutCell,
    system_functions,
};

pub const SAMPLE_RATE: u32 = 44100;
const MAX_SAMPLES: usize = 32768;

pub struct SampleQueue {
    buffer: [u8; MAX_SAMPLES],
    tail: Mutex<u16>,
    length: Mutex<u16>,
    process_exited: bool,
    // A sample that was possibly skipped while being added to the queue
    skipped_right: Option<u8>,
}

impl SampleQueue {
    fn new() -> Self {
        Self {
            buffer: [0; MAX_SAMPLES],
            tail: Mutex::new(0),
            length: Mutex::new(0),
            process_exited: false,
            skipped_right: None,
        }
    }

    fn push(&mut self, sample: u8) -> bool {
        let mut length = self.length.lock().unwrap();
        if *length == MAX_SAMPLES as u16 {
            return false;
        }

        let head = (*self.tail.lock().unwrap() + *length) % MAX_SAMPLES as u16;
        self.buffer[head as usize] = sample;
        *length += 1;

        true
    }

    fn push_sample(&mut self, left: u8, right: u8) -> bool {
        if let Some(skipped_right) = self.skipped_right {
            if self.push(skipped_right) {
                self.skipped_right = None;
            } else {
                return false;
            }
        }

        if !self.push(left) {
            self.skipped_right = Some(right);
            return true;
        }

        if !self.push(right) {
            return false;
        }

        true
    }
}

unsafe impl Send for SampleQueue {}
unsafe impl Sync for SampleQueue {}

impl Iterator for SampleQueue {
    type Item = rodio::Sample;

    fn next(&mut self) -> Option<Self::Item> {
        if self.process_exited {
            return None;
        }

        let mut length = self.length.lock().unwrap();
        if *length == 0 {
            return Some(0.0);
        }

        let mut tail = self.tail.lock().unwrap();

        let sample = self.buffer[*tail as usize];

        *tail = (*tail + 1) % MAX_SAMPLES as u16;
        *length -= 1;

        Some(sample as rodio::Sample / 255.0)
    }
}

impl rodio::Source for SampleQueue {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> rodio::ChannelCount {
        nz!(2)
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

pub struct ProcessAudioState {
    player: Player,
    sample_buffer: SampleQueue,
}

impl ProcessAudioState {
    pub fn new(player: Player) -> Self {
        let sample_buffer = SampleQueue::new();
        player.append(&sample_buffer);
        Self {
            player,
            sample_buffer,
        }
    }

    pub fn play(&mut self, samples: &[u8], left_volume: u8, right_volue: u8) -> usize {
        for (i, &sample) in samples.iter().enumerate() {
            let (left, right) = split_sample(sample, left_volume, right_volue);
            if !self.sample_buffer.push_sample(left, right) {
                return i;
            }
        }
        samples.len()
    }
}
