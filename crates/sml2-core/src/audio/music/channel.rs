//! Music channel state and bytecode decoding.

use super::tables::*;
use super::MusicWrite;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MusicChannel {
    pub id: u8,
    pub active: bool,
    pub section_ptr: u16,
    pub stream_ptr: u16,
    pub note_timer: u8,
    pub note_duration: u8,
    pub loop_ptr: u16,
    pub loop_count: u8,
    pub instrument: [u8; 3],
    pub freq_lo: u8,
    pub freq_hi: u8,
    pub rest: bool,
    pub suppressed: bool,
    pub env_timer: u8,
    pub env_counter: u8,
    pub vibrato_timer: u8,
    pub vibrato_flag: u8,
    pub slide_state: u8,
    pub slide_step: u8,
    pub bend_flag: u8,
    /// Set when a section word with a zero high byte ended the song ($115CD).
    /// The sequencer clears `sCurSong` and returns before the remaining channels
    /// or the envelope counters run, so the caller must stop the walk.
    pub song_ended: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelCommandResult {
    Continue,
    SetDurTable(u16),
    SetTranspose(i8),
}

impl MusicChannel {
    pub fn new(id: u8) -> Self {
        Self {
            id,
            active: false,
            section_ptr: 0,
            stream_ptr: 0,
            note_timer: 1,
            // `$10000`'s reset zeroes the whole `$A400-$A4FF` block, so a
            // channel that has not seen a `$Ax` duration command keeps 0 here,
            // not an invented default.
            note_duration: 0,
            loop_ptr: 0,
            loop_count: 0,
            instrument: [0; 3],
            freq_lo: 0,
            freq_hi: 0,
            rest: false,
            suppressed: false,
            env_timer: 0,
            env_counter: 0,
            vibrato_timer: 0,
            vibrato_flag: 0,
            slide_state: 0,
            slide_step: 0,
            bend_flag: 0,
            song_ended: false,
        }
    }

    /// Reset channel for a new section starting at `section_ptr`.
    pub fn init_section(&mut self, section_ptr: u16, bank: &[u8]) {
        self.active = section_ptr != 0;
        self.song_ended = false;
        self.section_ptr = section_ptr;
        self.loop_ptr = 0;
        self.loop_count = 0;
        self.note_timer = 1;
        self.env_counter = 0;

        if self.active {
            let stream = word_at(bank, self.section_ptr);
            self.section_ptr = self.section_ptr.wrapping_add(2);
            self.stream_ptr = stream;
        } else {
            self.stream_ptr = 0;
        }
    }

    /// Advances the channel by one frame.
    pub fn tick(
        &mut self,
        bank: &[u8],
        transpose: &mut i8,
        dur_table_ptr: &mut u16,
        drum_mode: &mut u8,
        sfx_active: bool,
        out: &mut Vec<MusicWrite>,
    ) {
        if !self.active {
            return;
        }

        self.note_timer = self.note_timer.wrapping_sub(1);
        if self.note_timer != 0 {
            // $11565: with six frames left, a wave-channel instrument whose
            // level byte has bit 7 set boosts NR32 to $40. The ROM checks this
            // before the rest/takeover gates, so an SFX never masks it.
            if self.id == 3 && (self.instrument[2] & 0x80) != 0 && self.note_timer == 6 {
                out.push(MusicWrite::Ch3Level(0x40));
            }
            // $1157C: a resting channel, and a channel an SFX has taken over,
            // skip the modulation and envelope calls for this frame.
            if self.rest || sfx_active {
                return;
            }
            self.tick_modulation(drum_mode, out);
            self.tick_software_envelope(bank, out);
            return;
        }

        // Note timer expired: step stream commands until a note is triggered or stream ends
        while self.active {
            let cmd = self.read_stream_byte(bank);
            match cmd {
                0x00 => {
                    // Section end: read next section pointer ($1159B). The ROM
                    // tests the HIGH byte only: $00 -> song end ($115CD),
                    // $FF -> section-list jump ($115B6), anything else is the
                    // next stream pointer.
                    let next_sec = word_at(bank, self.section_ptr);
                    self.section_ptr = self.section_ptr.wrapping_add(2);

                    if next_sec >> 8 == 0x00 {
                        // $115CD: sCurSong = 0; call $13F5E (stop all); ret.
                        self.song_ended = true;
                        break;
                    } else if next_sec >> 8 == 0xFF {
                        // Loop section
                        let target = word_at(bank, self.section_ptr);
                        self.section_ptr = target;
                        let stream = word_at(bank, self.section_ptr);
                        self.section_ptr = self.section_ptr.wrapping_add(2);
                        self.stream_ptr = stream;
                    } else {
                        self.stream_ptr = next_sec;
                    }
                }
                0x9D => {
                    // Set instrument
                    let b0 = self.read_stream_byte(bank);
                    let b1 = self.read_stream_byte(bank);
                    let b2 = self.read_stream_byte(bank);
                    self.instrument = [b0, b1, b2];

                    if self.id == 3 && !sfx_active {
                        let wave_ptr = ((b1 as u16) << 8) | (b0 as u16);
                        let wave_off = bank4_offset(wave_ptr);
                        if wave_off + 16 <= bank.len() {
                            let mut pattern = [0u8; 16];
                            pattern.copy_from_slice(&bank[wave_off..wave_off + 16]);
                            out.push(MusicWrite::Ch3Enable(0));
                            out.push(MusicWrite::Ch3WavePattern(pattern));
                        }
                    }
                }
                0x9E => {
                    // Set duration table pointer
                    let lo = self.read_stream_byte(bank);
                    let hi = self.read_stream_byte(bank);
                    *dur_table_ptr = ((hi as u16) << 8) | (lo as u16);
                }
                0x9F => {
                    // Set note transpose
                    let val = self.read_stream_byte(bank);
                    *transpose = val as i8;
                }
                0x9B => {
                    // Loop start
                    let count = self.read_stream_byte(bank);
                    self.loop_count = count;
                    self.loop_ptr = self.stream_ptr;
                }
                0x9C => {
                    // Loop end
                    if self.loop_count > 1 {
                        self.loop_count -= 1;
                        self.stream_ptr = self.loop_ptr;
                    } else {
                        self.loop_count = 0;
                    }
                }
                0x99 => {
                    *drum_mode = 1;
                }
                0x9A => {
                    *drum_mode = 0;
                }
                0x97 => self.slide_state = 1,
                0x98 => self.slide_state = 0,
                0x95 => self.bend_flag = 1,
                0x96 => self.bend_flag = 0,
                0xA0..=0xAF => {
                    // Duration selection
                    let dur_idx = (cmd & 0x0F) as usize;
                    let dur_off = bank4_offset(*dur_table_ptr) + dur_idx;
                    self.note_duration = if dur_off < bank.len() {
                        bank[dur_off]
                    } else {
                        0x18
                    };

                    // $116B9: the software envelope's fire value scales the note
                    // duration by instrument[1]'s high nibble ($80 = half,
                    // $40 = quarter, anything else = three quarters), with a
                    // minimum of 2; when zero, env_timer is not written.
                    let scale = self.instrument[1] & 0xF0;
                    if scale != 0 {
                        let scaled = if scale & 0x80 != 0 {
                            self.note_duration >> 1
                        } else if scale & 0x40 != 0 {
                            (self.note_duration >> 1) >> 1
                        } else {
                            (self.note_duration >> 1) + ((self.note_duration >> 1) >> 1)
                        };
                        self.env_timer = scaled.max(2);
                    }

                    // Next byte is the note pitch
                    let note = self.read_stream_byte(bank);
                    self.trigger_note(note, *transpose, drum_mode, sfx_active, bank, out);
                    break;
                }
                note @ 0x01..=0x94 => {
                    self.trigger_note(note, *transpose, drum_mode, sfx_active, bank, out);
                    break;
                }
                _ => {}
            }
        }
    }

    /// `$11817`/`$11839`: per-frame modulation while a note is held. A slide or
    /// a bend consumes the frame first; with neither active, the wave channel
    /// runs its drum duck (`$1199A`), whose counter shares `slide_step` with the
    /// ch3 portamento (`ram.asm` documents the overlap).
    fn tick_modulation(&mut self, drum_mode: &mut u8, out: &mut Vec<MusicWrite>) {
        if self.slide_state != 0 || self.bend_flag != 0 {
            return;
        }
        if self.id != 3 || *drum_mode == 0 || *drum_mode == 2 {
            return;
        }
        self.slide_step = self.slide_step.wrapping_add(1);
        let level = match self.slide_step {
            3 => Some(0x40),
            5 => Some(0x60),
            7 => Some(0x00),
            13 => {
                *drum_mode = 2; // $1199A: the duck has ended
                Some(0x00)
            }
            _ => None,
        };
        if let Some(level) = level {
            out.push(MusicWrite::Ch3Level(level));
        }
    }

    /// `$11899`/`$118E4`: when the channel's envelope counter reaches the value
    /// the last duration command computed, write `$5CAC[(instrument[1] & $0F) - 1]`
    /// to NRx2 and retrigger the note (NRx4 = freq_hi | $80).
    fn tick_software_envelope(&mut self, bank: &[u8], out: &mut Vec<MusicWrite>) {
        if self.id > 2 || self.instrument[1] == 0 {
            return;
        }
        if self.env_counter != self.env_timer {
            return;
        }
        let index = (self.instrument[1] & 0x0F).wrapping_sub(1) as u16;
        let off = bank4_offset(0x5CACu16.wrapping_add(index));
        let envelope = bank.get(off).copied().unwrap_or(0);
        match self.id {
            1 => {
                out.push(MusicWrite::Ch1Envelope(envelope));
                out.push(MusicWrite::Ch1FrequencyControl(self.freq_hi | 0x80));
            }
            2 => {
                out.push(MusicWrite::Ch2Envelope(envelope));
                out.push(MusicWrite::Ch2FrequencyControl(self.freq_hi | 0x80));
            }
            _ => {}
        }
    }

    fn read_stream_byte(&mut self, bank: &[u8]) -> u8 {
        let off = bank4_offset(self.stream_ptr);
        self.stream_ptr = self.stream_ptr.wrapping_add(1);
        if off < bank.len() {
            bank[off]
        } else {
            0
        }
    }

    fn trigger_note(
        &mut self,
        note: u8,
        transpose: i8,
        drum_mode: &mut u8,
        sfx_active: bool,
        bank: &[u8],
        out: &mut Vec<MusicWrite>,
    ) {
        self.note_timer = self.note_duration;
        // $117D9 zeroes block+14: every note restarts the envelope counter.
        self.env_counter = 0;

        // $11804: on channel 3 note trigger, if sMusicDrumMode != 0, reset to 1 and slide_step = 0
        if self.id == 3 && *drum_mode != 0 {
            *drum_mode = 1;
            self.slide_step = 0;
        }

        // $11728 takes the channel-4 branch BEFORE the note-$01 rest test, so a
        // noise note $01 reads its $5C06 record like any other note (and leaves
        // the rest flag alone).
        if self.id == 4 {
            if note == 0x1F {
                if !sfx_active {
                    out.push(MusicWrite::DrumTrigger);
                }
            } else {
                // ROM $11876: bc = note is byte offset into table $5C06; copies 5 bytes
                let off = bank4_offset(0x5C06) + (note as usize);
                if off + 5 <= bank.len() {
                    let rec = &bank[off..off + 5];
                    if !sfx_active {
                        // ROM $11777..$117E8:
                        // rec[2] -> rAUD4LEN ($FF20)
                        // rec[0] -> rAUD4ENV ($FF21)
                        // rec[3] -> rAUD4POLY ($FF22)
                        // rec[4] | 0x80 -> rAUD4CONTR ($FF23)
                        out.push(MusicWrite::Ch4Length(rec[2]));
                        out.push(MusicWrite::Ch4Envelope(rec[0]));
                        out.push(MusicWrite::Ch4Polynomial(rec[3]));
                        out.push(MusicWrite::Ch4Control(rec[4] | 0x80));
                    }
                }
            }
            return;
        }

        if note == 0x01 {
            // Rest ($11765 / $117D0): sets channel rest flag and silences envelope
            self.rest = true;
            if !sfx_active {
                match self.id {
                    1 => {
                        out.push(MusicWrite::Ch1Sweep(0));
                        out.push(MusicWrite::Ch1DutyLength(self.instrument[2] & 0xC0));
                        out.push(MusicWrite::Ch1Envelope(0x08)); // Volume 0
                        out.push(MusicWrite::Ch1FrequencyLow(self.freq_lo));
                        out.push(MusicWrite::Ch1FrequencyControl(self.freq_hi | 0x80));
                    }
                    2 => {
                        out.push(MusicWrite::Ch2DutyLength(self.instrument[2] & 0xC0));
                        out.push(MusicWrite::Ch2Envelope(0x08)); // Volume 0
                        out.push(MusicWrite::Ch2FrequencyLow(self.freq_lo));
                        out.push(MusicWrite::Ch2FrequencyControl(self.freq_hi | 0x80));
                    }
                    3 => {
                        out.push(MusicWrite::Ch3Enable(0x80));
                        out.push(MusicWrite::Ch3Length(0));
                        out.push(MusicWrite::Ch3Level(0)); // Volume 0
                        out.push(MusicWrite::Ch3FrequencyLow(self.freq_lo));
                        out.push(MusicWrite::Ch3FrequencyControl(self.freq_hi | 0x80));
                    }
                    _ => {}
                }
            }
            return;
        }

        self.rest = false;

        // Channels 1, 2, 3: pitch calculation per ROM $11755..$11762
        // Stream note bytes are byte offsets into table $5AF4.
        // Signed transpose is added directly to the byte offset in SM83:
        // hl = $5AF4 + (note as i16) + (transpose as i16)
        let addr = 0x5AF4u16
            .wrapping_add((note as u16).wrapping_add(transpose as i16 as u16));
        let off = bank4_offset(addr);
        let freq = if off + 1 < bank.len() {
            (bank[off] as u16) | ((bank[off + 1] as u16) << 8)
        } else {
            0
        };
        self.freq_lo = (freq & 0xFF) as u8;
        self.freq_hi = (freq >> 8) as u8;

        if sfx_active {
            return;
        }

        match self.id {
            1 => {
                out.push(MusicWrite::Ch1Sweep(0));
                out.push(MusicWrite::Ch1DutyLength(self.instrument[2] & 0xC0));
                out.push(MusicWrite::Ch1Envelope(self.instrument[0]));
                out.push(MusicWrite::Ch1FrequencyLow(self.freq_lo));
                out.push(MusicWrite::Ch1FrequencyControl(self.freq_hi | 0x80));
            }
            2 => {
                out.push(MusicWrite::Ch2DutyLength(self.instrument[2] & 0xC0));
                out.push(MusicWrite::Ch2Envelope(self.instrument[0]));
                out.push(MusicWrite::Ch2FrequencyLow(self.freq_lo));
                out.push(MusicWrite::Ch2FrequencyControl(self.freq_hi | 0x80));
            }
            3 => {
                out.push(MusicWrite::Ch3Enable(0x80));
                out.push(MusicWrite::Ch3Length(0));
                out.push(MusicWrite::Ch3Level(self.instrument[2] & 0x60));
                out.push(MusicWrite::Ch3FrequencyLow(self.freq_lo));
                out.push(MusicWrite::Ch3FrequencyControl(self.freq_hi | 0x80));
            }
            _ => {}
        }
    }
}

#[inline]
pub fn word_at(bank: &[u8], addr: u16) -> u16 {
    let off = bank4_offset(addr);
    if off + 1 < bank.len() {
        (bank[off] as u16) | ((bank[off + 1] as u16) << 8)
    } else {
        0
    }
}
