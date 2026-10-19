//! Music sequencing engine, song playback, and 4-channel command processing (ROM $112E0..$118A0).

pub mod channel;
pub mod tables;

#[cfg(test)]
mod rom_model;
#[cfg(test)]
mod tests;

pub use channel::MusicChannel;
pub use tables::{bank4_offset, FAST_MUSIC_SONGS, PITCH_TABLE, SONG_POINTERS, SOUND_BANK04};

use channel::word_at;

/// Ordered register and hardware events emitted by the 4-channel music sequencer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicWrite {
    // Channel 1 (Pulse 1)
    Ch1Sweep(u8),
    Ch1DutyLength(u8),
    Ch1Envelope(u8),
    Ch1FrequencyLow(u8),
    Ch1FrequencyControl(u8),

    // Channel 2 (Pulse 2)
    Ch2DutyLength(u8),
    Ch2Envelope(u8),
    Ch2FrequencyLow(u8),
    Ch2FrequencyControl(u8),

    // Channel 3 (Wave)
    Ch3Enable(u8),
    Ch3Length(u8),
    Ch3Level(u8),
    Ch3FrequencyLow(u8),
    Ch3FrequencyControl(u8),
    Ch3WavePattern([u8; 16]),

    // Channel 4 (Noise)
    Ch4Length(u8),
    Ch4Envelope(u8),
    Ch4Polynomial(u8),
    Ch4Control(u8),

    // Drum kick/snare trigger (triggers Noise SFX 3 via $A478)
    DrumTrigger,
}

/// Native 4-channel music sequencing engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicSequencer {
    pub cur_song: u8,
    pub fast_music: bool,
    pub transpose: i8,
    pub dur_table_ptr: u16,
    pub drum_mode: u8,
    pub channels: [MusicChannel; 4],
}

impl Default for MusicSequencer {
    fn default() -> Self {
        Self {
            cur_song: 0,
            fast_music: false,
            transpose: 0,
            dur_table_ptr: 0x5C61,
            drum_mode: 0,
            channels: [
                MusicChannel::new(1),
                MusicChannel::new(2),
                MusicChannel::new(3),
                MusicChannel::new(4),
            ],
        }
    }
}

impl MusicSequencer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cur_song(&self) -> u8 {
        self.cur_song
    }

    pub fn is_playing(&self) -> bool {
        self.cur_song != 0
    }

    /// Stops music playback immediately and resets channels per ROM $13F5E.
    pub fn stop(&mut self) {
        self.cur_song = 0;
        self.fast_music = false;
        self.drum_mode = 0;
        for ch in &mut self.channels {
            ch.active = false;
        }
    }

    /// Starts playing a song by 1-based song ID (1..=42) per ROM $112E0 and $1145A.
    pub fn start_song(&mut self, song_id: u8, bank: &[u8]) {
        if song_id == 0 || song_id as usize > SONG_POINTERS.len() {
            return;
        }

        self.cur_song = song_id;
        if song_id == 9 {
            self.fast_music = true;
        } else if [2, 3, 12, 15, 24].contains(&song_id) {
            self.fast_music = false;
        }

        let ptr = SONG_POINTERS[(song_id - 1) as usize];
        let off = bank4_offset(ptr);
        if off + 13 > bank.len() {
            return;
        }

        let base_transpose = bank[off] as i8;
        self.transpose = if self.fast_music {
            base_transpose.wrapping_add(4)
        } else {
            base_transpose
        };

        let dur_norm = word_at(bank, ptr.wrapping_add(1));
        let dur_fast = word_at(bank, ptr.wrapping_add(3));
        self.dur_table_ptr = if self.fast_music { dur_fast } else { dur_norm };

        let ch1_sec = word_at(bank, ptr.wrapping_add(5));
        let ch2_sec = word_at(bank, ptr.wrapping_add(7));
        let ch3_sec = word_at(bank, ptr.wrapping_add(9));
        let ch4_sec = word_at(bank, ptr.wrapping_add(11));

        self.channels[0].init_section(ch1_sec, bank);
        self.channels[1].init_section(ch2_sec, bank);
        self.channels[2].init_section(ch3_sec, bank);
        self.channels[3].init_section(ch4_sec, bank);
    }

    /// Advances the music sequencer by one frame per ROM $11638.
    pub fn tick(&mut self, bank: &[u8], sfx_active: [u8; 3], out: &mut Vec<MusicWrite>) {
        if self.cur_song == 0 {
            return;
        }

        // Channel 1: Pulse 1 (masked if Slot 0 SFX is active)
        self.channels[0].tick(
            bank,
            &mut self.transpose,
            &mut self.dur_table_ptr,
            &mut self.drum_mode,
            sfx_active[0] != 0,
            out,
        );
        if self.song_ended_on(0) {
            return;
        }

        // Channel 2: Pulse 2 (unmasked)
        self.channels[1].tick(
            bank,
            &mut self.transpose,
            &mut self.dur_table_ptr,
            &mut self.drum_mode,
            false,
            out,
        );
        if self.song_ended_on(1) {
            return;
        }

        // Channel 3: Wave (masked if Slot 1 SFX is active)
        self.channels[2].tick(
            bank,
            &mut self.transpose,
            &mut self.dur_table_ptr,
            &mut self.drum_mode,
            sfx_active[1] != 0,
            out,
        );
        if self.song_ended_on(2) {
            return;
        }

        // Channel 4: Noise (masked if Slot 2 SFX is active)
        self.channels[3].tick(
            bank,
            &mut self.transpose,
            &mut self.dur_table_ptr,
            &mut self.drum_mode,
            sfx_active[2] != 0,
            out,
        );
        if self.song_ended_on(3) {
            return;
        }

        // Increment software envelope counter per frame for Ch1, Ch2, Ch3 ($11808)
        self.channels[0].env_counter = self.channels[0].env_counter.wrapping_add(1);
        self.channels[1].env_counter = self.channels[1].env_counter.wrapping_add(1);
        self.channels[2].env_counter = self.channels[2].env_counter.wrapping_add(1);
    }

    /// `$115CD`: a channel whose section word has a zero high byte ends the
    /// whole song; the sequencer returns immediately, so the channels after it
    /// are not ticked this frame and the `$11808` envelope counters do not run.
    fn song_ended_on(&mut self, idx: usize) -> bool {
        if self.channels[idx].song_ended {
            self.cur_song = 0;
            true
        } else {
            false
        }
    }
}
