pub use crate::settings::Volume;
use crate::settings::atomic_write;
use sml2_core::{
    audio::{Apu, apu::CHANNEL_WEIGHT_UNIT},
    front_end::{SAVE_IMAGE_LEN, SLOT_LEN, SaveSlots},
    player_sim::PlayerKeys,
};
use sml2_runtime::{
    FrameInput, Session, audio_output::AudioOutput, content::SessionAssets,
    divider_clock::VirtualDividerClock, session::SessionState,
};
use std::{fs, path::PathBuf};

pub struct Game {
    pub session: Session,
    save: Option<PathBuf>,
    /// Why the save file that exists at `save` was not loaded. While set,
    /// nothing is written there, so unloaded progress is never replaced.
    save_blocked: Option<String>,
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
        let mut save_blocked = None;

        match fs::read(&save) {
            Ok(data) if data.len() == SAVE_IMAGE_LEN => {
                slots = SaveSlots::from_image(&data);
                let valid = slots.validate_and_init();
                if let Some(slot) = valid.iter().position(|&v| v) {
                    if let Some(record) = slots.load_slot(slot) {
                        if session.apply_save_record(&record) {
                            session.active_slot = Some(slot as u8);
                        } else {
                            save_blocked = Some(format!("pipe {slot} could not be resumed"));
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
                    save_blocked = Some("the legacy save could not be resumed".into());
                }
            }
            Ok(data) => {
                save_blocked = Some(format!(
                    "{} bytes is not a {SAVE_IMAGE_LEN}-byte slot image",
                    data.len()
                ))
            }
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                save_blocked = Some(e.to_string())
            }
            _ => {}
        }
        if let Some(reason) = &save_blocked {
            diagnostics.push(format!("Save: {reason}; not overwriting it"));
        }
        session.slots = slots;

        let mut game = Self::with_session(session, Some(save), audio_enabled, diagnostics);
        game.save_blocked = save_blocked;
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
            save_blocked: None,
            apu: Apu::new(),
            buffer: Vec::new(),
            output: None,
            audio_enabled,
            div_clock: VirtualDividerClock::seeded_from_time(),
            diagnostics,
        }
    }

    /// Write the three-slot image (`$F0` bytes) atomically. Refuses while a
    /// save that was not loaded sits at the save path.
    fn persist_slots(&self) -> Result<(), std::io::Error> {
        if let Some(reason) = &self.save_blocked {
            return Err(std::io::Error::other(format!(
                "the existing save could not be loaded ({reason}); not overwriting it"
            )));
        }
        match &self.save {
            Some(save) => atomic_write(save, &self.session.slots.image()),
            None => Ok(()),
        }
    }

    /// Why progress is not being saved, when the existing save file could not
    /// be loaded.
    pub fn save_blocked(&self) -> Option<&str> {
        self.save_blocked.as_deref()
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

    /// Mario is in a level and off the floor, where Down starts a spin jump.
    pub fn airborne(&self) -> bool {
        matches!(&self.session.state, SessionState::InLevel(ctx) if !ctx.sim.on_ground)
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
        // The sink's rate decides the tick's length: WASAPI and CoreAudio often
        // run at 48 kHz while the pipe backends ask for 44.1 kHz.
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
