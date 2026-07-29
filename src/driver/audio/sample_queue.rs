use std::{num::NonZeroU32, ops::Deref, sync::Mutex};

use rodio::nz;

use crate::mut_cell::MutCell;

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
    pub fn new() -> Self {
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

impl<T: Iterator + 'static> Iterator for MutCell<T> {
    type Item = T::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.borrow_static().next()
    }
}

impl<T: rodio::Source + 'static> rodio::Source for MutCell<T> {
    fn current_span_len(&self) -> Option<usize> {
        self.deref().current_span_len()
    }

    fn channels(&self) -> rodio::ChannelCount {
        self.deref().channels()
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        self.deref().sample_rate()
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        self.deref().total_duration()
    }
}
