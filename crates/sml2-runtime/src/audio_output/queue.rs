//! Bounded playback queue: a slow sink must not retain old gameplay audio.

use super::LIVE_FRAME_SAMPLES;
use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};

// Keep at most one tick (16.7 ms) of stereo PCM in this queue: on a hitch the
// newest tick wins, so the stream never plays audio the game has moved past.
const MAX_SAMPLES: usize = 2 * LIVE_FRAME_SAMPLES;

#[derive(Default)]
struct Pending {
    chunks: VecDeque<Vec<f32>>,
    samples: usize,
    closed: bool,
}

#[derive(Default)]
pub(super) struct SampleQueue {
    pending: Mutex<Pending>,
    ready: Condvar,
}

impl SampleQueue {
    pub(super) fn push(&self, mut samples: Vec<f32>) {
        samples.truncate(samples.len() & !1);
        if samples.is_empty() {
            return;
        }
        if samples.len() > MAX_SAMPLES {
            samples.drain(..samples.len() - MAX_SAMPLES);
        }
        let mut pending = self.pending.lock().unwrap();
        if pending.closed {
            return;
        }
        while pending.samples + samples.len() > MAX_SAMPLES {
            let stale = pending.chunks.pop_front().expect("queued sample count");
            pending.samples -= stale.len();
        }
        pending.samples += samples.len();
        pending.chunks.push_back(samples);
        self.ready.notify_one();
    }

    pub(super) fn pop(&self) -> Option<Vec<f32>> {
        let mut pending = self.pending.lock().unwrap();
        loop {
            if pending.closed {
                return None;
            }
            if let Some(samples) = pending.chunks.pop_front() {
                pending.samples -= samples.len();
                return Some(samples);
            }
            pending = self.ready.wait(pending).unwrap();
        }
    }

    pub(super) fn close(&self) {
        let mut pending = self.pending.lock().unwrap();
        pending.closed = true;
        pending.chunks.clear();
        pending.samples = 0;
        self.ready.notify_all();
    }
}

#[cfg(test)]
#[path = "tests/queue.rs"]
mod tests;
