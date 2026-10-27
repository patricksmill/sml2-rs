use crate::settings::atomic_write;
pub use crate::settings::Volume;
use sml2_core::{
    audio::{apu::CHANNEL_WEIGHT_UNIT, Apu},
    front_end::{SaveSlots, SAVE_IMAGE_LEN, SLOT_LEN},
    player_sim::PlayerKeys,
};
use sml2_runtime::{
    audio_output::AudioOutput, content::SessionAssets, divider_clock::VirtualDividerClock,
    FrameInput, Session,
};
use std::{fs, path::PathBuf};

pub struct Game {
    pub session: Session,
    save: Option<PathBuf>,
    apu: Apu,
    buffer: Vec<f32>,
    output: Option<AudioOutput>,
    audio_enabled: bool,
    div_clock: VirtualDividerClock,
    pub diagnostics: Vec<String>,
}

impl Game {
    pub fn new(assets: SessionAssets, save: PathBuf, audio_enabled: bool) -> Self {
        let mut session = Session::new(assets);
        let mut diagnostics = Vec::new();
        let mut slots = SaveSlots::new();
        let mut migrated = false;

        match fs::read(&save) {
            Ok(data) if data.len() == SAVE_IMAGE_LEN => {
                slots = SaveSlots::from_image(&data);
                let valid = slots.validate_and_init();
                if let Some(slot) = valid.iter().position(|&v| v) {
                    if let Some(record) = slots.load_slot(slot) {
                        if session.apply_save_record(&record) {
                            session.active_slot = Some(slot as u8);
                        }
                    }
                }
            }
            Ok(data) if data.len() == SLOT_LEN => {
                // Pre-slot single-record saves migrate into pipe A.
                if slots.write_slot(0, &data) && session.apply_save_record(&data) {
                    session.active_slot = Some(0);
                    migrated = true;
                } else {
                    diagnostics.push("Save data could not be loaded".into());
                }
            }
            Ok(data) => diagnostics.push(format!(
                "Save: {} bytes is not a {SAVE_IMAGE_LEN}-byte slot image",
                data.len()
            )),
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                diagnostics.push(format!("Save: {e}"))
            }
            _ => {}
        }
        session.slots = slots;

        let mut game = Self::with_session(session, Some(save), audio_enabled, diagnostics);
        // Rewrite a migrated legacy save in the slot image immediately, so the
        // next launch and the file select see the same three pipes.
        if migrated {
            if let Err(e) = game.persist_slots() {
                game.diagnostics.push(format!("Save: {e}"));
            }
        }
        game
    }

    /// An isolated authoring session with no save location or progress IO.
    pub fn preview(assets: SessionAssets, audio_enabled: bool) -> Self {
        Self::with_session(Session::new(assets), None, audio_enabled, Vec::new())
    }

    fn with_session(
        session: Session,
        save: Option<PathBuf>,
        audio_enabled: bool,
        diagnostics: Vec<String>,
    ) -> Self {
        Self {
            session,
            save,
            apu: Apu::new(),
            buffer: Vec::new(),
            output: None,
            audio_enabled,
            div_clock: VirtualDividerClock::seeded_from_time(),
            diagnostics,
        }
    }

    /// Write the three-slot image (`$F0` bytes) atomically.
    fn persist_slots(&self) -> Result<(), std::io::Error> {
        match &self.save {
            Some(save) => atomic_write(save, &self.session.slots.image()),
            None => Ok(()),
        }
    }

    /// Write the live progress record into the active pipe and persist the
    /// slot image. The debug panel's Save button uses this; edits are not
    /// written until it is pressed.
    pub fn save_now(&mut self) -> std::io::Result<()> {
        if self.save.is_none() {
            return Ok(());
        }
        if let Some(slot) = self.session.active_slot {
            let record = self.session.save_record();
            self.session.slots.write_slot(slot as usize, &record);
        }
        self.persist_slots()
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

    pub fn tick(&mut self, keys: PlayerKeys, volume: impl Into<Volume>) {
        let volume = volume.into();
        self.div_clock.advance_tick();
        let div = self.div_clock.sample();
        let effects = self.session.advance(FrameInput { keys, div });
        if effects.persist_slots {
            if let Err(e) = self.persist_slots() {
                self.diagnostics.push(format!("Save: {e}"));
            }
        }
        self.diagnostics.extend(effects.diagnostics);
        self.apu.ingest_driver(&self.session.sound_driver);
        let effects = self.session.sound_driver.effect_channels();
        self.apu.channel_weights = effects.map(|effect| {
            let level = if effect { volume.effects } else { volume.music };
            (level.clamp(0.0, 1.0) * CHANNEL_WEIGHT_UNIT as f32).round() as u16
        });
        // Each catch-up tick advances its own PCM interval before the next
        // driver update, so short SFX cannot disappear into a zero-sample tick.
        // The sink's rate decides the tick's length: WASAPI shared mode often
        // runs at 48 kHz while the pipe backends ask for 44.1 kHz.
        let rate = self.output.as_ref().map_or(44_100, |o| o.sample_rate());
        let samples = rate as usize / 60;
        self.buffer.clear();
        self.apu.generate_samples(rate, samples, &mut self.buffer);
        for sample in &mut self.buffer {
            *sample *= volume.master;
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
