use std::{
    num::NonZeroU32,
    sync::{Arc, Mutex, Weak},
};

use rodio::nz;

pub const SAMPLE_RATE: u32 = 44100;
const MAX_SAMPLES: usize = 32768;

pub struct SampleQueue {
    buffer: [u8; MAX_SAMPLES],
    tail: u16,
    length: u16,
    // A sample that was possibly skipped while being added to the queue
    skipped_right: Option<u8>,
}

impl SampleQueue {
    pub fn new() -> Self {
        Self {
            buffer: [0; MAX_SAMPLES],
            tail: 0,
            length: 0,
            skipped_right: None,
        }
    }

    pub fn new_arc() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::new()))
    }

    fn push(&mut self, sample: u8) -> bool {
        if self.length == MAX_SAMPLES as u16 {
            return false;
        }

        let head = (self.tail + self.length) % MAX_SAMPLES as u16;
        self.buffer[head as usize] = sample;
        self.length += 1;

        true
    }

    /// Pushes a two-channel audio sample, making sure not to drop any samples when there is no
    /// space in the queue
    pub fn push_sample(&mut self, left: u8, right: u8) -> bool {
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
        if self.length == 0 {
            return Some(0.0);
        }

        let sample = self.buffer[self.tail as usize];

        self.tail = (self.tail + 1) % MAX_SAMPLES as u16;
        self.length -= 1;

        Some(sample as rodio::Sample / 255.0)
    }
}

pub struct WeakWrapper(Weak<Mutex<SampleQueue>>);

impl WeakWrapper {
    pub fn new(original: &Arc<Mutex<SampleQueue>>) -> Self {
        Self(Arc::downgrade(original))
    }
}

impl Iterator for WeakWrapper {
    type Item = rodio::Sample;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.upgrade()?.lock().ok()?.next()
    }
}

impl rodio::Source for WeakWrapper {
    fn current_span_len(&self) -> Option<usize> {
        if self.0.upgrade().is_some() {
            None
        } else {
            Some(0)
        }
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
