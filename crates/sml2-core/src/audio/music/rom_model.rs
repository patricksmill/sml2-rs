//! Test-only transliteration of the bank-4 music walk.
//!
//! This is the oracle for the sequencer tests: a straight reading of
//! `$11638` (channel walk), `$1159B` (section end / list jump), `$11510`
//! (`$9D`-`$9F`, `$95`-`$9C`) and `$11728` (note trigger) over the real song
//! data in `SOUND_BANK04`. It models only the state the port must reproduce
//! byte-for-byte — section pointer, stream pointer, note timer/duration,
//! frequency cache and rest flag — not the APU writes.

use super::tables::{bank4_offset, SONG_POINTERS};

const PITCH_TABLE: u16 = 0x5AF4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RomChannel {
    /// `block+0/+1`: the section-list entry whose stream is playing.
    pub section: u16,
    /// `block+2`: note/command countdown.
    pub timer: u8,
    /// `block+3`: note duration.
    pub duration: u8,
    /// `block+4/+5`: stream pointer.
    pub stream: u16,
    /// `block+9/+10`: frequency cache.
    pub freq_lo: u8,
    pub freq_hi: u8,
    /// `block+11`: rest flag (note byte `$01`).
    pub rest: u8,
    /// `block+15` low 7 bits: loop count (`$9B` ORs it in, `$9C` decrements).
    pub loops: u8,
    /// `block+12/+13`: loop-return stream pointer (`$9B`).
    pub loop_ptr: u16,
}

impl RomChannel {
    pub fn active(&self) -> bool {
        self.section >> 8 != 0
    }
}

pub struct RomSeq {
    pub cur: u8,
    pub transpose: u8,
    pub dur: u16,
    pub channels: [RomChannel; 4],
}

#[inline]
fn read(addr: u16) -> u8 {
    let off = bank4_offset(addr);
    super::SOUND_BANK04.get(off).copied().unwrap_or(0)
}

#[inline]
fn word(addr: u16) -> u16 {
    read(addr) as u16 | ((read(addr.wrapping_add(1)) as u16) << 8)
}

impl RomSeq {
    /// `$112E0` + `StartMusic $1145A`: load the header, then the first stream
    /// word of each channel's section list. `fast_music` is state on the
    /// console too, so the caller replays songs in the order it wants.
    pub fn new(song: u8, fast_music: bool) -> Self {
        let ptr = SONG_POINTERS[(song - 1) as usize];
        let mut transpose = read(ptr);
        if fast_music {
            transpose = transpose.wrapping_add(4);
        }
        let dur = if fast_music {
            word(ptr.wrapping_add(3))
        } else {
            word(ptr.wrapping_add(1))
        };
        let mut channels = [RomChannel::default(); 4];
        for (i, ch) in channels.iter_mut().enumerate() {
            // `StartMusic` stores the header word at `block+0/+1` and `$11449`
            // reads the stream pointer through it; the timer is set to 1 by the
            // `$114EB` loop.
            let section = word(ptr.wrapping_add(5 + 2 * i as u16));
            *ch = RomChannel {
                section,
                timer: 1,
                stream: if section >> 8 != 0 { word(section) } else { 0 },
                ..RomChannel::default()
            };
        }
        Self {
            cur: song,
            transpose,
            dur,
            channels,
        }
    }

    fn read_stream(ch: &mut RomChannel) -> u8 {
        let b = read(ch.stream);
        ch.stream = ch.stream.wrapping_add(1);
        b
    }

    /// One `$11638` frame.
    pub fn tick(&mut self) {
        if self.cur == 0 {
            return;
        }
        for idx in 0..4 {
            if !self.channels[idx].active() {
                continue;
            }
            self.channels[idx].timer = self.channels[idx].timer.wrapping_sub(1);
            if self.channels[idx].timer != 0 {
                continue;
            }
            let mut guard = 0u32;
            while self.channels[idx].active() {
                guard += 1;
                assert!(
                    guard < 4096,
                    "song {} channel {} command loop",
                    self.cur,
                    idx + 1
                );
                let cmd = Self::read_stream(&mut self.channels[idx]);
                match cmd {
                    0x00 => {
                        // $1159B: advance the section pointer, read the next
                        // word, branch on its HIGH byte.
                        self.channels[idx].section = self.channels[idx].section.wrapping_add(2);
                        let w = word(self.channels[idx].section);
                        if w >> 8 == 0x00 {
                            self.cur = 0; // $115CD: stop all, return
                            return;
                        }
                        if w >> 8 == 0xFF {
                            // $115B6: the following word is the new section.
                            let target = word(self.channels[idx].section.wrapping_add(2));
                            self.channels[idx].section = target;
                            self.channels[idx].stream = word(target);
                        } else {
                            self.channels[idx].stream = w;
                        }
                    }
                    0x9D => {
                        let _ = Self::read_stream(&mut self.channels[idx]);
                        let _ = Self::read_stream(&mut self.channels[idx]);
                        let _ = Self::read_stream(&mut self.channels[idx]);
                    }
                    0x9E => {
                        let lo = Self::read_stream(&mut self.channels[idx]);
                        let hi = Self::read_stream(&mut self.channels[idx]);
                        self.dur = lo as u16 | ((hi as u16) << 8);
                    }
                    0x9F => self.transpose = Self::read_stream(&mut self.channels[idx]),
                    0x9B => {
                        let count = Self::read_stream(&mut self.channels[idx]);
                        self.channels[idx].loops |= count;
                        self.channels[idx].loop_ptr = self.channels[idx].stream;
                    }
                    0x9C => {
                        let ch = &mut self.channels[idx];
                        ch.loops = ch.loops.wrapping_sub(1);
                        if ch.loops & 0x7F != 0 {
                            ch.stream = ch.loop_ptr;
                        }
                    }
                    0x95..=0x9A => {}
                    0xA0..=0xAF => {
                        let dur_index = (cmd & 0x0F) as u16;
                        let duration = read(self.dur.wrapping_add(dur_index));
                        let note = Self::read_stream(&mut self.channels[idx]);
                        self.trigger(idx, note, Some(duration));
                        break;
                    }
                    note => {
                        self.trigger(idx, note, None);
                        break;
                    }
                }
            }
        }
    }

    fn trigger(&mut self, idx: usize, note: u8, duration: Option<u8>) {
        let ch = &mut self.channels[idx];
        if let Some(d) = duration {
            ch.duration = d;
        }
        ch.timer = ch.duration;
        if idx == 3 {
            // $11728 branches to $1176A before the note-$01 test: noise notes
            // are $5C06 offsets and never touch the rest flag.
            return;
        }
        if note == 0x01 {
            ch.rest = 1;
            return;
        }
        ch.rest = 0;
        // $11755: hl = note + signed transpose, then a flat add into the table.
        let off = PITCH_TABLE
            .wrapping_add((note as u16).wrapping_add(self.transpose as i8 as i16 as u16));
        ch.freq_lo = read(off);
        ch.freq_hi = read(off.wrapping_add(1));
    }
}
