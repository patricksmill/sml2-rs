//! Live audio playback stream for `sml2-runtime`.
//!
//! The sink is platform-specific: Linux shells out to PulseAudio (`pacat`) or
//! PipeWire (`pw-play`) and feeds raw stereo float32 PCM through stdin; Windows
//! opens a WASAPI shared-mode stream through `cpal`. Both backends accept the
//! same interleaved stereo `f32` frames from the APU.

/// Stereo sample frames per tick for the live frontends' fixed 60 Hz simulation.
pub const LIVE_FRAME_SAMPLES: usize = 44_100 / 60;

/// Soft limiter shared by the live backends: a gain boost with smooth
/// saturation so low-level SFX (coins, jumps) stay audible while loud
/// multi-channel music never clips.
pub(crate) fn limit(sample: f32) -> f32 {
    (sample * 2.2).tanh()
}

#[cfg(not(windows))]
mod queue;

#[cfg(not(windows))]
mod pipe;
#[cfg(not(windows))]
pub use pipe::AudioOutput;

#[cfg(windows)]
mod wasapi;
#[cfg(windows)]
pub use wasapi::AudioOutput;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_output_open_or_skip() {
        if let Some(out) = AudioOutput::open() {
            out.write_samples(&vec![0.0; 2 * out.sample_rate() as usize / 60]);
        }
    }
}
