//! Linux live playback: a `pacat`/`pw-play` child process fed through stdin.
//!
//! Samples pass through a smooth soft-limiter to provide comfortable, rich
//! loudness without digital clipping.

use super::queue::SampleQueue;
use super::LIVE_FRAME_SAMPLES;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

/// Interleaved floats per sink write (a quarter tick, ~4.2 ms). Keeps a
/// backend's server-side buffer from absorbing a whole tick as one unit.
const WRITE_SLICE_SAMPLES: usize = 2 * (LIVE_FRAME_SAMPLES / 4);

pub struct AudioOutput {
    queue: Arc<SampleQueue>,
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

        let queue = Arc::new(SampleQueue::default());
        let playback = Arc::clone(&queue);

        let worker = thread::Builder::new()
            .name("sml2-audio-output".into())
            .spawn(move || {
                let mut byte_buf = Vec::with_capacity(4096);
                'stream: while let Some(samples) = playback.pop() {
                    // Hand the sink sub-tick slices: a whole 16.7 ms write can
                    // sit in the server queue as one unit and add its full
                    // length to the playback delay on some backends.
                    for piece in samples.chunks(WRITE_SLICE_SAMPLES) {
                        byte_buf.clear();
                        byte_buf.reserve(piece.len() * 4);
                        for &s in piece {
                            let val = super::limit(s);
                            byte_buf.extend_from_slice(&val.to_ne_bytes());
                        }
                        if stdin.write_all(&byte_buf).is_err() {
                            break 'stream;
                        }
                    }
                }
                playback.close();
                drop(stdin);
                let _ = child.wait();
            })
            .ok()?;

        println!("Live audio enabled via {name}");
        Some(Self {
            queue,
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
                "--latency-msec=15",
            ]);
            if !pulse_env_set && wslg_socket.exists() {
                cmd.env("PULSE_SERVER", "/mnt/wslg/PulseServer");
            }
            return Some((cmd, "PulseAudio (pacat)"));
        }

        // Try pw-play (PipeWire)
        if let Ok(path) = which("pw-play") {
            let mut cmd = Command::new(path);
            cmd.args(["-", "-p", "--rate=44100", "--channels=2", "--format=f32"]);
            return Some((cmd, "PipeWire (pw-play)"));
        }

        None
    }

    /// The pipe backend always asks the child for 44.1 kHz.
    pub fn sample_rate(&self) -> u32 {
        44_100
    }

    /// Enqueue a frame of stereo interleaved float32 samples to the audio stream.
    pub fn write_samples(&self, samples: &[f32]) {
        self.queue.push(samples.to_vec());
    }
}

impl Drop for AudioOutput {
    fn drop(&mut self) {
        self.queue.close();
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
