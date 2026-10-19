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
            note_duration: 0x18,
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
        }
    }

    /// Reset channel for a new section starting at `section_ptr`.
    pub fn init_section(&mut self, section_ptr: u16, bank: &[u8]) {
        self.active = section_ptr != 0 && section_ptr != 0x7F52;
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
            // Mid-note modulation: Channel 3 level boost when 6 frames remain ($11565)
            if self.id == 3 && (self.instrument[2] & 0x80) != 0 && self.note_timer == 6 {
                if !sfx_active {
                    out.push(MusicWrite::Ch3Level(0x40));
                }
            }
            return;
        }

        // Note timer expired: step stream commands until a note is triggered or stream ends
        while self.active {
            let cmd = self.read_stream_byte(bank);
            match cmd {
                0x00 => {
                    // Section end: read next section pointer
                    let next_sec = word_at(bank, self.section_ptr);
                    self.section_ptr = self.section_ptr.wrapping_add(2);

                    if next_sec == 0 {
                        self.active = false;
                        break;
                    } else if next_sec == 0xFFFF {
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
                    if self.id == 3 {
                        *drum_mode = 1;
                    } else {
                        self.vibrato_flag = 1;
                    }
                }
                0x9A => {
                    if self.id == 3 {
                        *drum_mode = 0;
                    } else {
                        self.vibrato_flag = 0;
                    }
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

                    // Next byte is the note pitch
                    let note = self.read_stream_byte(bank);
                    self.trigger_note(note, *transpose, sfx_active, out);
                    break;
                }
                note @ 0x01..=0x94 => {
                    self.trigger_note(note, *transpose, sfx_active, out);
                    break;
                }
                _ => {}
            }
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
        sfx_active: bool,
        out: &mut Vec<MusicWrite>,
    ) {
        self.note_timer = self.note_duration;

        if note == 0x01 {
            // Rest
            self.rest = true;
            return;
        }

        self.rest = false;

        if self.id == 4 {
            if note == 0x1F {
                if !sfx_active {
                    out.push(MusicWrite::DrumTrigger);
                }
            } else {
                let inst_idx = (note as usize).min(NOISE_INSTRUMENTS.len() - 1);
                let rec = NOISE_INSTRUMENTS[inst_idx];
                if !sfx_active {
                    out.push(MusicWrite::Ch4Length(rec[0]));
                    out.push(MusicWrite::Ch4Envelope(rec[1]));
                    out.push(MusicWrite::Ch4Polynomial(rec[3]));
                    out.push(MusicWrite::Ch4Control(rec[4] | 0x80));
                }
            }
            return;
        }

        // Channels 1, 2, 3: pitch calculation
        let idx = ((note as i16) + (transpose as i16)).clamp(0, (PITCH_TABLE.len() - 1) as i16) as usize;
        let freq = PITCH_TABLE[idx];
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
