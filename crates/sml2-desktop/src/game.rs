use crate::settings::atomic_write;
use sml2_core::{audio::Apu, divider::Divider, player_sim::PlayerKeys};
use sml2_runtime::{
    audio_output::{AudioOutput, LIVE_FRAME_SAMPLES},
    content::SessionAssets,
    FrameInput, Session,
};
use std::{fs, path::PathBuf, time::Instant};

pub struct Game {
    pub session: Session,
    save: PathBuf,
    apu: Apu,
    buffer: Vec<f32>,
    output: Option<AudioOutput>,
    audio_enabled: bool,
    origin: Instant,
    pub diagnostics: Vec<String>,
}

impl Game {
    pub fn new(assets: SessionAssets, save: PathBuf, audio_enabled: bool) -> Self {
        let mut session = Session::new(assets);
        let mut diagnostics = Vec::new();
        match fs::read(&save) {
            Ok(data) if !session.apply_save_record(&data) => {
                diagnostics.push("Save data could not be loaded".into())
            }
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                diagnostics.push(format!("Save: {e}"))
            }
            _ => {}
        }
        Self {
            session,
            save,
            apu: Apu::new(),
            buffer: Vec::new(),
            output: None,
            audio_enabled,
            origin: Instant::now(),
            diagnostics,
        }
    }

    pub fn audio_running(&mut self, running: bool) {
        if running && self.audio_enabled && self.output.is_none() {
            self.output = AudioOutput::open();
            // Do not repeatedly retry a missing backend on every repaint.
            if self.output.is_none() {
                self.audio_enabled = false;
                self.diagnostics
                    .push("Live audio output is unavailable on this system".into());
            }
        } else if !running {
            self.output = None;
        }
    }

    pub fn tick(&mut self, keys: PlayerKeys, volume: f32) {
        let div = Divider::from_elapsed(0, self.origin.elapsed()).sample();
        let effects = self.session.advance(FrameInput { keys, div });
        for record in effects.saves {
            if let Err(e) = atomic_write(&self.save, &record) {
                self.diagnostics.push(format!("Save: {e}"));
            }
        }
        self.diagnostics.extend(effects.diagnostics);
        self.apu.ingest_driver(&self.session.sound_driver);
        // Each catch-up tick advances its own PCM interval before the next
        // driver update, so short SFX cannot disappear into a zero-sample tick.
        let samples = LIVE_FRAME_SAMPLES;
        self.buffer.clear();
        self.apu.generate_samples(44100, samples, &mut self.buffer);
        for sample in &mut self.buffer {
            *sample *= volume;
        }
        if let Some(output) = &self.output {
            output.write_samples(&self.buffer);
        }
        // Diagnostic lists are visible in the debug panel, with bounded host memory.
        if self.diagnostics.len() > 64 {
            self.diagnostics.drain(..self.diagnostics.len() - 64);
        }
    }
}

#[cfg(test)]
mod tests;
