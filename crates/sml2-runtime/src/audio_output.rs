//! Live audio playback stream for `sml2-runtime`.
//!
//! Spawns a background PulseAudio client (`pacat`) or PipeWire client (`pw-play`)
//! and feeds it raw stereo float32 PCM samples in real time via stdin.
//! Samples pass through a smooth soft-limiter to provide comfortable, rich loudness
//! without digital clipping.

use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::mpsc::{sync_channel, SyncSender, TrySendError};
use std::thread::{self, JoinHandle};

pub struct AudioOutput {
    sender: Option<SyncSender<Vec<f32>>>,
    worker: Option<JoinHandle<()>>,
}

impl AudioOutput {
    /// Attempts to open a live audio playback stream.
    /// Returns None if no compatible audio system is detected.
    pub fn open() -> Option<Self> {
        let (mut cmd, name) = Self::detect_backend()?;

        cmd.stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("note: failed to spawn {name} for live audio ({e})");
                return None;
            }
        };

        let mut stdin = child.stdin.take()?;

        // Bounded queue: enough to ride out a render hitch, shallow enough that
        // the stream stays close to real time (6 frames = 100 ms).
        let (tx, rx) = sync_channel::<Vec<f32>>(6);

        let worker = thread::Builder::new()
            .name("sml2-audio-output".into())
            .spawn(move || {
                let mut byte_buf = Vec::with_capacity(4096);
                while let Ok(samples) = rx.recv() {
                    byte_buf.clear();
                    byte_buf.reserve(samples.len() * 4);
                    for &s in &samples {
                        // Soft limiter: gain boost with smooth saturation
                        // Low-level SFX (coins, jumps) get brought up comfortably,
                        // while loud multi-channel music saturates gracefully.
                        let val = (s * 2.2).tanh();
                        byte_buf.extend_from_slice(&val.to_ne_bytes());
                    }
                    if stdin.write_all(&byte_buf).is_err() {
                        break;
                    }
                }
                drop(stdin);
                let _ = child.wait();
            })
            .ok()?;

        println!("Live audio enabled via {name}");
        Some(Self {
            sender: Some(tx),
            worker: Some(worker),
        })
    }

    fn detect_backend() -> Option<(Command, &'static str)> {
        let pulse_env_set = std::env::var_os("PULSE_SERVER").is_some();
        let wslg_socket = std::path::Path::new("/mnt/wslg/PulseServer");

        // Try pacat first (standard PulseAudio raw stream client)
        if let Ok(path) = which("pacat") {
            let mut cmd = Command::new(path);
            cmd.args([
                "--playback",
                "--format=float32ne",
                "--rate=44100",
                "--channels=2",
                "--latency-msec=30",
            ]);
            if !pulse_env_set && wslg_socket.exists() {
                cmd.env("PULSE_SERVER", "/mnt/wslg/PulseServer");
            }
            return Some((cmd, "PulseAudio (pacat)"));
        }

        // Try pw-play (PipeWire)
        if let Ok(path) = which("pw-play") {
            let mut cmd = Command::new(path);
            cmd.args([
                "-",
                "-p",
                "--rate=44100",
                "--channels=2",
                "--format=f32",
            ]);
            return Some((cmd, "PipeWire (pw-play)"));
        }

        None
    }

    /// Enqueue a frame of stereo interleaved float32 samples to the audio stream.
    pub fn write_samples(&self, samples: &[f32]) {
        if let Some(tx) = &self.sender {
            match tx.try_send(samples.to_vec()) {
                Ok(()) => {}
                Err(TrySendError::Full(_)) => {
                    // Buffer is full (e.g. game hitch or pause); drop to avoid latency drift
                }
                Err(TrySendError::Disconnected(_)) => {}
            }
        }
    }
}

impl Drop for AudioOutput {
    fn drop(&mut self) {
        self.sender.take(); // Close channel
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
    }
}

fn which(binary: &str) -> Result<String, ()> {
    if let Ok(path) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join(binary);
            if candidate.is_file() {
                return Ok(candidate.to_string_lossy().into_owned());
            }
        }
    }
    for dir in ["/usr/bin", "/usr/sbin", "/usr/local/bin"] {
        let candidate = std::path::Path::new(dir).join(binary);
        if candidate.is_file() {
            return Ok(candidate.to_string_lossy().into_owned());
        }
    }
    Err(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_output_open_or_skip() {
        if let Some(out) = AudioOutput::open() {
            out.write_samples(&[0.0; 1470]);
        }
    }
}
