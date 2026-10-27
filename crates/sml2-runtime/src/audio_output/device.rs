//! Windows and macOS live playback: the default output device through `cpal`
//! (WASAPI shared mode on Windows, CoreAudio on macOS).
//!
//! The device's default mix format decides the sample rate, so the APU is
//! generated at that rate (see [`AudioOutput::sample_rate`]) instead of being
//! forced to 44.1 kHz. The audio callback drains a bounded queue; on a hitch
//! the oldest PCM is dropped so playback never runs behind the game.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub struct AudioOutput {
    // Kept alive for as long as playback should run; dropping it stops the
    // stream.
    _stream: cpal::Stream,
    pending: Arc<Mutex<VecDeque<f32>>>,
    rate: u32,
}

impl AudioOutput {
    /// Opens the default output device (shared mode on Windows).
    /// Returns None when there is no device or its format is unsupported.
    pub fn open() -> Option<Self> {
        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let supported = device.default_output_config().ok()?;
        let rate = supported.sample_rate().0;
        let channels = supported.channels() as usize;
        let config: cpal::StreamConfig = supported.config();

        // Two ticks of interleaved stereo is the most we ever queue.
        let cap = (rate as usize / 60) * 4;
        let pending = Arc::new(Mutex::new(VecDeque::with_capacity(cap)));

        let stream = match supported.sample_format() {
            SampleFormat::F32 => {
                let q = Arc::clone(&pending);
                device
                    .build_output_stream(
                        &config,
                        move |data: &mut [f32], _| fill_f32(data, channels, &q),
                        |err| eprintln!("sml2 audio: {err}"),
                        None,
                    )
                    .ok()?
            }
            SampleFormat::I16 => {
                let q = Arc::clone(&pending);
                device
                    .build_output_stream(
                        &config,
                        move |data: &mut [i16], _| fill_i16(data, channels, &q),
                        |err| eprintln!("sml2 audio: {err}"),
                        None,
                    )
                    .ok()?
            }
            SampleFormat::U16 => {
                let q = Arc::clone(&pending);
                device
                    .build_output_stream(
                        &config,
                        move |data: &mut [u16], _| fill_u16(data, channels, &q),
                        |err| eprintln!("sml2 audio: {err}"),
                        None,
                    )
                    .ok()?
            }
            other => {
                eprintln!("note: unsupported {} sample format {other:?}", host.id().name());
                return None;
            }
        };
        stream.play().ok()?;
        println!(
            "Live audio enabled via {} ({rate} Hz, {channels} ch)",
            host.id().name()
        );
        Some(Self {
            _stream: stream,
            pending,
            rate,
        })
    }

    /// The mix rate the callback runs at; the host generates this many
    /// samples per 60 Hz tick.
    pub fn sample_rate(&self) -> u32 {
        self.rate
    }

    /// Enqueue one tick of interleaved stereo float32 samples.
    pub fn write_samples(&self, samples: &[f32]) {
        let samples = &samples[..samples.len() & !1];
        let mut pending = self.pending.lock().unwrap();
        let cap = (self.rate as usize / 60) * 4;
        if samples.len() >= cap {
            let start = samples.len() - cap;
            pending.clear();
            pending.extend(samples[start..].iter().map(|&s| super::limit(s)));
            return;
        }
        while pending.len() + samples.len() > cap {
            pending.pop_front();
        }
        pending.extend(samples.iter().map(|&s| super::limit(s)));
    }
}

/// Pull one stereo pair per device frame; pad with silence on underrun.
fn next_pair(queue: &mut VecDeque<f32>) -> (f32, f32) {
    let l = queue.pop_front().unwrap_or(0.0);
    let r = queue.pop_front().unwrap_or(0.0);
    (l, r)
}

fn fill_f32(data: &mut [f32], channels: usize, pending: &Mutex<VecDeque<f32>>) {
    let mut queue = pending.lock().unwrap();
    for frame in data.chunks_mut(channels) {
        let (l, r) = next_pair(&mut queue);
        if channels == 1 {
            frame[0] = 0.5 * (l + r);
        } else {
            frame[0] = l;
            frame[1] = r;
            for s in &mut frame[2..] {
                *s = 0.0;
            }
        }
    }
}

fn fill_i16(data: &mut [i16], channels: usize, pending: &Mutex<VecDeque<f32>>) {
    let mut queue = pending.lock().unwrap();
    for frame in data.chunks_mut(channels) {
        let (l, r) = next_pair(&mut queue);
        let to_pcm = |v: f32| (v.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        if channels == 1 {
            frame[0] = to_pcm(0.5 * (l + r));
        } else {
            frame[0] = to_pcm(l);
            frame[1] = to_pcm(r);
            for s in &mut frame[2..] {
                *s = 0;
            }
        }
    }
}

fn fill_u16(data: &mut [u16], channels: usize, pending: &Mutex<VecDeque<f32>>) {
    let mut queue = pending.lock().unwrap();
    for frame in data.chunks_mut(channels) {
        let (l, r) = next_pair(&mut queue);
        let to_pcm = |v: f32| ((v.clamp(-1.0, 1.0) * 0.5 + 0.5) * u16::MAX as f32) as u16;
        if channels == 1 {
            frame[0] = to_pcm(0.5 * (l + r));
        } else {
            frame[0] = to_pcm(l);
            frame[1] = to_pcm(r);
            for s in &mut frame[2..] {
                *s = 32_768;
            }
        }
    }
}
