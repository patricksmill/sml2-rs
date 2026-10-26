//! Existing CLI host: clock policy, persistence, audio output and window loop.
//! New frontends can instead drive `Session::advance` and consume its effects.

use crate::divider_clock::VirtualDividerClock;
use crate::persistence::atomic_write;
use crate::play_tape::PlayTape;
use crate::session::{Boot, FrameInput, Session, SessionAssets, StepOutcome};
use minifb::{Key, Window, WindowOptions};
use sml2_core::{
    audio::apu::Apu,
    front_end::{SaveSlots, SAVE_IMAGE_LEN, SLOT_LEN},
    player_sim::PlayerKeys,
};
use std::{
    error::Error,
    fs,
    ops::{Deref, DerefMut},
    path::PathBuf,
};

/// PCM duration of one live simulation tick. Catch-up ticks each retain their
/// own audio interval instead of assigning a wall-clock deficit to the first.
pub struct AudioClock {
    rate: u32,
}

impl AudioClock {
    pub fn new(rate: u32) -> Self {
        Self { rate }
    }

    pub fn take(&mut self) -> usize {
        self.rate as usize / 60
    }
}

/// Compatibility host for the current interactive and headless CLI.
/// Field access to simulation state is forwarded for existing callers.
pub struct SessionRunner {
    pub session: Session,
    pub save_path: PathBuf,
    pub virtual_clock: VirtualDividerClock,
    pub vw: usize,
    pub vh: usize,
    pub scale: usize,
    pub headless: bool,
    pub audio_sync: Option<AudioClock>,
    pub apu: Apu,
    pub audio_buffer: Vec<f32>,
    pub audio_out_path: Option<PathBuf>,
    pub recorded_audio: Vec<f32>,
    pub live_audio: Option<crate::audio_output::AudioOutput>,
    save_error: Option<std::io::Error>,
}

impl Deref for SessionRunner {
    type Target = Session;
    fn deref(&self) -> &Session {
        &self.session
    }
}
impl DerefMut for SessionRunner {
    fn deref_mut(&mut self) -> &mut Session {
        &mut self.session
    }
}

impl SessionRunner {
    pub fn new(
        assets: SessionAssets,
        save_path: PathBuf,
        scale: usize,
        view: (usize, usize),
        headless: bool,
    ) -> Self {
        Self::new_with_boot(assets, save_path, scale, view, headless, Boot::Overworld)
    }

    pub fn new_with_boot(
        assets: SessionAssets,
        save_path: PathBuf,
        scale: usize,
        view: (usize, usize),
        headless: bool,
        boot: Boot,
    ) -> Self {
        let mut runner = Self {
            session: Session::new(assets),
            save_path,
            virtual_clock: if headless {
                VirtualDividerClock::new(0)
            } else {
                VirtualDividerClock::seeded_from_time()
            },
            vw: view.0,
            vh: view.1,
            scale,
            headless,
            audio_sync: None,
            apu: Apu::new(),
            audio_buffer: Vec::with_capacity(1470),
            audio_out_path: None,
            recorded_audio: Vec::new(),
            live_audio: if headless {
                None
            } else {
                crate::audio_output::AudioOutput::open()
            },
            save_error: None,
        };
        runner.load_from_disk();
        if boot == Boot::Title {
            runner.session.enter_title();
        } else {
            runner.session.active_slot.get_or_insert(0);
        }
        if let Some(out) = &runner.live_audio {
            runner.audio_sync = Some(AudioClock::new(out.sample_rate()));
        }
        runner
    }

    pub fn load_from_disk(&mut self) -> bool {
        let data = match fs::read(&self.save_path) {
            Ok(data) if matches!(data.len(), SLOT_LEN | SAVE_IMAGE_LEN) => data,
            Ok(data) => {
                eprintln!(
                    "Save {}: unsupported length {}",
                    self.save_path.display(),
                    data.len()
                );
                return false;
            }
            Err(error) => {
                if error.kind() != std::io::ErrorKind::NotFound {
                    eprintln!("Save {}: {error}", self.save_path.display());
                }
                return false;
            }
        };
        let mut slots = SaveSlots::from_image(&data);
        let valid = slots.validate_and_init();
        let Some(slot) = valid.iter().position(|&valid| valid) else {
            self.session.slots = slots;
            return false;
        };
        let record = slots.load_slot(slot).unwrap();
        if !self.session.apply_save_record(&record) {
            return false;
        }
        self.session.slots = slots;
        self.session.active_slot = Some(slot as u8);
        if data.len() == SLOT_LEN {
            if let Err(error) = atomic_write(&self.save_path, &self.session.slots.image()) {
                self.report_save_error(error);
            }
        }
        true
    }

    pub fn save_to_disk(&self) -> Result<(), std::io::Error> {
        if !self.session.saveable() {
            return Ok(());
        }
        let mut slots = self.session.slots.clone();
        slots.write_slot(
            self.session.active_slot.unwrap_or(0) as usize,
            &self.session.save_record(),
        );
        atomic_write(&self.save_path, &slots.image())
    }

    fn report_save_error(&mut self, error: std::io::Error) {
        let error = std::io::Error::new(
            error.kind(),
            format!("Save {}: {error}", self.save_path.display()),
        );
        eprintln!("{error}");
        self.save_error = Some(error);
    }

    pub fn render_frame(&self) -> Vec<u8> {
        self.session.render_frame((self.vw, self.vh))
    }

    /// The CLI clock and device policy lives here, outside simulation.
    pub fn step_frame(&mut self, keys: PlayerKeys) -> StepOutcome {
        self.virtual_clock.advance_tick();
        let div = self.virtual_clock.sample();
        let effects = self.session.advance(FrameInput { keys, div });
        if let Some(record) = effects.saves.last() {
            self.session
                .slots
                .write_slot(self.session.active_slot.unwrap_or(0) as usize, record);
        }
        if effects.persist_slots {
            if let Err(error) = atomic_write(&self.save_path, &self.session.slots.image()) {
                self.report_save_error(error);
            }
        }
        for diagnostic in effects.diagnostics {
            eprintln!("{diagnostic}");
        }
        self.audio_buffer.clear();
        self.apu.ingest_driver(&self.session.sound_driver);
        // Match live PCM to the 60 Hz host; preserve console cadence for exports.
        let samples = match self.audio_sync.as_mut() {
            Some(clock) => clock.take(),
            None => self.apu.frame_sample_count(44100),
        };
        self.apu
            .generate_samples(44100, samples, &mut self.audio_buffer);
        if let Some(audio) = &self.live_audio {
            audio.write_samples(&self.audio_buffer);
        }
        if self.audio_out_path.is_some() {
            self.recorded_audio.extend_from_slice(&self.audio_buffer);
        }
        effects.outcome
    }

    /// Returns the stereo audio buffer generated during the most recent stepped frame.
    #[inline]
    pub fn audio_samples(&self) -> &[f32] {
        &self.audio_buffer
    }

    /// Save all accumulated recorded audio to `audio_out_path` if configured.
    pub fn save_recorded_audio(&self) -> Result<(), Box<dyn Error>> {
        if let Some(path) = &self.audio_out_path {
            if !self.recorded_audio.is_empty() {
                sml2_core::audio::write_wav_file(path, &self.recorded_audio, 44100)?;
            }
        }
        Ok(())
    }

    /// Run the interactive session loop in a window.
    pub fn run_window(&mut self, tape: Option<PlayTape>) -> Result<(), Box<dyn Error>> {
        let mut window = Window::new(
            "sml2 — overworld session (arrows move, Z jump/A, X run/B, ENTER start/pause, Q quits)",
            self.vw * self.scale,
            self.vh * self.scale,
            WindowOptions {
                resize: true,
                ..WindowOptions::default()
            },
        )
        .map_err(|e| format!("cannot open window ({e})"))?;
        window.set_target_fps(60);

        let mut buf = vec![0u32; self.vw * self.scale * self.vh * self.scale];
        let mut tape_i = 0;

        while window.is_open() && !window.is_key_down(Key::Escape) && !window.is_key_down(Key::Q) {
            let held = |k: Key| window.is_key_down(k);
            let mut keys = PlayerKeys {
                right: held(Key::Right),
                left: held(Key::Left),
                up: held(Key::Up),
                down: held(Key::Down),
                a: held(Key::Z) || held(Key::Space),
                b: held(Key::X) || held(Key::LeftShift),
                start: held(Key::Enter),
                select: held(Key::Backspace),
            };

            if let Some(t) = tape.as_ref() {
                if let Some(k) = t.frame(tape_i) {
                    keys = k;
                    tape_i += 1;
                }
            }

            self.step_frame(keys);
            if let Some(error) = self.save_error.take() {
                return Err(error.into());
            }

            let frame = self.render_frame();
            let (win_w, win_h) = window.get_size();
            if win_w == 0 || win_h == 0 {
                continue;
            }
            crate::present::letterbox_gray(&frame, (self.vw, self.vh), (win_w, win_h), &mut buf);

            window
                .update_with_buffer(&buf, win_w, win_h)
                .map_err(|e| e.to_string())?;
        }

        self.save_recorded_audio()?;
        Ok(())
    }

    /// Run the session loop in headless mode for a fixed number of frames.
    pub fn run_headless(
        &mut self,
        max_frames: u64,
        tape: Option<PlayTape>,
    ) -> Result<u64, Box<dyn Error>> {
        let mut tape_i = 0;
        for _ in 0..max_frames {
            let mut keys = PlayerKeys::default();
            if let Some(t) = tape.as_ref() {
                if let Some(k) = t.frame(tape_i) {
                    keys = k;
                    tape_i += 1;
                } else {
                    break;
                }
            }
            self.step_frame(keys);
            if let Some(error) = self.save_error.take() {
                return Err(error.into());
            }
        }
        self.save_recorded_audio()?;
        Ok(self.frame_n)
    }
}

#[cfg(test)]
mod tests;
