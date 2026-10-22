use crate::settings::atomic_write;
use sml2_core::{
    audio::Apu,
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
    save: PathBuf,
    isolated_preview: bool,
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

        let mut game = Self {
            session,
            save,
            isolated_preview: false,
            apu: Apu::new(),
            buffer: Vec::new(),
            output: None,
            audio_enabled,
            div_clock: VirtualDividerClock::seeded_from_time(),
            diagnostics,
        };
        // Rewrite a migrated legacy save in the slot image immediately, so the
        // next launch and the file select see the same three pipes.
        if migrated {
            if let Err(e) = game.persist_slots() {
                game.diagnostics.push(format!("Save: {e}"));
            }
        }
        game
    }

    /// Construct without opening or migrating any save file.
    pub fn preview(assets: SessionAssets, audio_enabled: bool) -> Result<Self, String> {
        let mut session = Session::new(assets);
        session.active_slot = None;
        session.debug_warp_level(0)?;
        // Populate the small-Mario sprite state before the first preview frame.
        session.advance(FrameInput::default());
        Ok(Self {
            session,
            save: PathBuf::new(),
            isolated_preview: true,
            apu: Apu::new(),
            buffer: Vec::new(),
            output: None,
            audio_enabled,
            div_clock: VirtualDividerClock::seeded_from_time(),
            diagnostics: Vec::new(),
        })
    }

    pub fn preview_finished(&self) -> bool {
        self.isolated_preview
            && match &self.session.state {
                sml2_runtime::session::SessionState::InLevel(ctx) => {
                    ctx.sim.dead != 0 || ctx.sim.exit_completed
                }
                _ => true,
            }
    }

    /// Write the three-slot image (`$F0` bytes) atomically.
    fn persist_slots(&self) -> Result<(), std::io::Error> {
        if self.isolated_preview {
            return Ok(());
        }
        atomic_write(&self.save, &self.session.slots.image())
    }

    /// Write the live progress record into the active pipe and persist the
    /// slot image. The debug panel's Save button uses this; edits are not
    /// written until it is pressed.
    pub fn save_now(&mut self) -> std::io::Result<()> {
        if self.isolated_preview {
            return Err(std::io::Error::other(
                "DX preview does not use progress saves",
            ));
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

    pub fn tick(&mut self, keys: PlayerKeys, volume: f32) {
        self.div_clock.advance_tick();
        let div = self.div_clock.sample();
        let effects = self.session.advance(FrameInput { keys, div });
        let saved = !effects.saves.is_empty();
        if saved {
            // The session already wrote the record into `active_slot`; persist
            // the complete image so the other pipes survive.
            if let Err(e) = self.persist_slots() {
                self.diagnostics.push(format!("Save: {e}"));
            }
        }
        self.diagnostics.extend(effects.diagnostics);
        self.apu.ingest_driver(&self.session.sound_driver);
        // Each catch-up tick advances its own PCM interval before the next
        // driver update, so short SFX cannot disappear into a zero-sample tick.
        // The sink's rate decides the tick's length: WASAPI shared mode often
        // runs at 48 kHz while the pipe backends ask for 44.1 kHz.
        let rate = self.output.as_ref().map_or(44_100, |o| o.sample_rate());
        let samples = rate as usize / 60;
        self.buffer.clear();
        self.apu.generate_samples(rate, samples, &mut self.buffer);
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
