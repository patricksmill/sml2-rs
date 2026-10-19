//! Native sound driver and channel dispatcher for Super Mario Land 2.
//!
//! Replaces SM83 jump tables and raw sound registers with typed, deterministic
//! sound effect dispatch and channel state management per ROM `$10030`.
//!
//! Ground truth references:
//! - Root update: ROM `$10030` (`main.asm:9411..9460`)
//! - Common exit & request clear: ROM `$10067` (`main.asm:9446..9453`)
//! - 1-based table lookup: ROM `$111A6` (`main.asm:10966..10977`)
//! - Slot 0 (Pulse 1) dispatch: ROM `$100DE` (`main.asm:9469..9512`)
//! - Slot 1 (Wave) dispatch: ROM `$106D3` (`main.asm:10054..10081`)
//! - Slot 2 (Noise) dispatch: ROM `$10E90` (`main.asm:10706..10729`)

#[cfg(test)]
mod tests;

use crate::audio::music::{MusicSequencer, MusicWrite};
use crate::audio::noise::{NoiseEffectState, NoiseMusicFlags, NoiseWrite};
use crate::audio::pulse::{PulseEffectState, PulseMusicFlags, PulseMusicState, PulseWrite};
use crate::audio::wave::{WaveEffectState, WaveMusicFlags, WaveMusicState, WaveWrite};

/// Master volume settings derived from sLowTimeFlag ($A264).
pub const MASTER_VOLUME_NORMAL: u8 = 0x77; // 119
pub const MASTER_VOLUME_LOW_TIME: u8 = 0x33; // 51

/// Slot 0 (Pulse 1) request IDs that bypass the channel busy check ($100EC..$1010A).
pub const SLOT0_BUSY_EXCEPTIONS: [u8; 8] = [0x09, 0x0C, 0x0F, 0x10, 0x11, 0x13, 0x14, 0x17];

/// Slot 1 (Wave) request ID that bypasses the channel busy check ($106DB).
pub const SLOT1_BUSY_EXCEPTION: u8 = 0x14; // 20 decimal

/// Slot 0 (Pulse 1) Start pointers at ROM $1007E (24 entries).
pub const SLOT0_START_TABLE: [u16; 24] = [
    0x4127, 0x413A, 0x415E, 0x4188, 0x41A9, 0x41F3, 0x4217, 0x4238, 0x4271, 0x42B0, 0x42B2, 0x42E2,
    0x434E, 0x438E, 0x43DC, 0x4408, 0x4465, 0x446B, 0x449C, 0x44D8, 0x4560, 0x459B, 0x45F1, 0x462C,
];

/// Slot 0 (Pulse 1) Tick pointers at ROM $100AE (24 entries).
pub const SLOT0_TICK_TABLE: [u16; 24] = [
    0x412D, 0x4140, 0x416A, 0x4193, 0x41B4, 0x41F9, 0x421D, 0x423E, 0x427F, 0x42B1, 0x42B8, 0x42F0,
    0x435C, 0x4399, 0x43E5, 0x4419, 0x44AA, 0x4471, 0x44AA, 0x44DE, 0x456E, 0x45A1, 0x45FF, 0x463A,
];

/// Slot 1 (Wave) Start pointers at ROM $106FA (24 entries).
pub const SLOT1_START_TABLE: [u16; 24] = [
    0x475A, 0x480A, 0x489C, 0x48F7, 0x4932, 0x4994, 0x49F9, 0x4A34, 0x4AAF, 0x4AEA, 0x4AEA, 0x4B95,
    0x4BC2, 0x4C2F, 0x4B1A, 0x4C48, 0x4C72, 0x4C8F, 0x4E42, 0x4CE1, 0x4D38, 0x4D6C, 0x4DAE, 0x4DEB,
];

/// Slot 1 (Wave) Tick pointers at ROM $1072A (24 entries).
pub const SLOT1_TICK_TABLE: [u16; 24] = [
    0x4768, 0x4813, 0x48AA, 0x4908, 0x4943, 0x49B2, 0x4A0A, 0x4A45, 0x4AC0, 0x4AF6, 0x4AF6, 0x4B9D,
    0x4BD6, 0x4C3B, 0x4B2B, 0x4C5B, 0x4C78, 0x4C98, 0x4E42, 0x4CF5, 0x4D41, 0x4D7D, 0x4DC2, 0x4D41,
];

/// Slot 2 (Noise) Start pointers at ROM $10E64 (11 entries).
pub const SLOT2_START_TABLE: [u16; 11] = [
    0x4EB3, 0x4EEB, 0x4F4B, 0x4F6A, 0x4F82, 0x4FC5, 0x5008, 0x5030, 0x5086, 0x50DC, 0x511C,
];

/// Slot 2 (Noise) Tick pointers at ROM $10E7A (11 entries).
pub const SLOT2_TICK_TABLE: [u16; 11] = [
    0x4EB9, 0x4EF6, 0x4F56, 0x4F76, 0x4F88, 0x4FCB, 0x5014, 0x5036, 0x508C, 0x50E7, 0x5122,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SfxLookupError {
    ZeroId,
    IdOutOfBounds { id: u8, max: u8 },
}

impl std::fmt::Display for SfxLookupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroId => write!(f, "sfx pointer lookup requires a nonzero 1-based ID"),
            Self::IdOutOfBounds { id, max } => {
                write!(f, "sfx ID {id} out of table bounds (max {max})")
            }
        }
    }
}

impl std::error::Error for SfxLookupError {}

/// Resolve a 1-based pointer lookup in a 16-bit word table per ROM $111A6.
#[inline]
pub fn lookup_sfx_pointer(table: &[u16], id: u8) -> Result<u16, SfxLookupError> {
    if id == 0 {
        return Err(SfxLookupError::ZeroId);
    }
    let idx = (id as usize) - 1;
    if idx >= table.len() {
        return Err(SfxLookupError::IdOutOfBounds {
            id,
            max: table.len() as u8,
        });
    }
    Ok(table[idx])
}

/// Channel/slot state for Wave (Slot 1).
pub type SfxSlotState = WaveEffectState;

/// Complete native sound driver state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoundDriver {
    music_bank: Vec<u8>,
    pub sfx_request_0: u8,
    pub sfx_request_1: u8,
    pub sfx_request_2: u8,
    pub music_request: u8,
    pub active_song: u8,
    pub sound_mode: u8,
    pub sound_paused: bool,
    pub low_time_flag: bool,
    pub stomp_chain: u8,
    pub master_volume: u8,
    pub pulse_effect: PulseEffectState,
    pub pulse_music: PulseMusicState,
    pub wave_slot: WaveEffectState,
    pub wave_music: WaveMusicState,
    pub noise_slot: NoiseEffectState,
    pub noise_music: NoiseMusicFlags,
    pub music_sequencer: MusicSequencer,
    pub pulse_writes: Vec<PulseWrite>,
    pub wave_writes: Vec<WaveWrite>,
    pub noise_writes: Vec<NoiseWrite>,
    pub music_writes: Vec<MusicWrite>,
    pub update_count: u32,
}

impl Default for SoundDriver {
    fn default() -> Self {
        Self {
            #[cfg(test)]
            music_bank: crate::audio::music::SOUND_BANK04.to_vec(),
            #[cfg(not(test))]
            music_bank: Vec::new(),
            sfx_request_0: 0,
            sfx_request_1: 0,
            sfx_request_2: 0,
            music_request: 0,
            active_song: 0,
            sound_mode: 0,
            sound_paused: false,
            low_time_flag: false,
            stomp_chain: 0,
            master_volume: MASTER_VOLUME_NORMAL,
            pulse_effect: PulseEffectState::default(),
            pulse_music: PulseMusicState::default(),
            wave_slot: WaveEffectState::default(),
            wave_music: WaveMusicState::default(),
            noise_slot: NoiseEffectState::default(),
            noise_music: NoiseMusicFlags::default(),
            music_sequencer: MusicSequencer::default(),
            pulse_writes: Vec::new(),
            wave_writes: Vec::new(),
            noise_writes: Vec::new(),
            music_writes: Vec::new(),
            update_count: 0,
        }
    }
}

impl SoundDriver {
    /// Construct a driver without music content. Use `with_music_bank` for songs.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a driver using caller-loaded Bank 4 data; no filesystem access.
    pub fn with_music_bank(music_bank: Vec<u8>) -> Self {
        Self {
            music_bank,
            ..Self::default()
        }
    }

    /// Clears all four sound request registers and the sound mode per ROM $10067.
    pub fn clear_requests(&mut self) {
        self.sfx_request_0 = 0;
        self.music_request = 0;
        self.sfx_request_1 = 0;
        self.sfx_request_2 = 0;
        self.sound_mode = 0;
    }

    /// Dispatch Slot 0 (Pulse 1 / Channel 1) per ROM $100DE.
    fn dispatch_slot_0(&mut self) {
        if self.sfx_request_0 != 0 {
            let req = self.sfx_request_0;
            let bypass_busy =
                SLOT0_BUSY_EXCEPTIONS.contains(&req) || self.pulse_music.vibrato_timer != 0;

            if !bypass_busy && self.pulse_effect.busy != 0 {
                return;
            }

            if matches!(req, 12 | 15 | 17) {
                // $42E2/$43DC/$4465 open with `call $13F5E`: stop the song and
                // run $13F6B's engine reset before the effect's own setup.
                self.music_sequencer.stop();
                self.reset_engine();
            }

            self.pulse_effect.request = req;
            self.pulse_effect.start(
                req,
                &mut self.pulse_music,
                self.stomp_chain,
                &mut self.pulse_writes,
            );
        } else if self.pulse_effect.active_effect != 0 {
            self.pulse_effect.tick(
                &mut self.pulse_music,
                self.stomp_chain,
                &mut self.pulse_writes,
            );
        }
    }

    /// Dispatch Slot 1 (Wave / Channel 3) per ROM $106D3.
    fn dispatch_slot_1(&mut self) {
        if self.sfx_request_1 != 0 {
            let req = self.sfx_request_1;
            let bypass_busy = req == SLOT1_BUSY_EXCEPTION;

            if !bypass_busy && self.wave_slot.busy != 0 {
                return;
            }

            self.wave_slot.request = req;
            self.wave_slot
                .start(req, &mut self.wave_music, &mut self.wave_writes);
        } else if self.wave_slot.active_effect != 0 {
            self.wave_slot
                .tick(&mut self.wave_music, &mut self.wave_writes);
        }
    }

    /// Dispatch Slot 2 (Noise / Channel 4) per ROM $10E90.
    fn dispatch_slot_2(&mut self) {
        if self.sfx_request_2 != 0 {
            let req = self.sfx_request_2;
            if self.noise_slot.busy != 0 {
                return;
            }

            self.noise_slot.request = req;
            self.noise_slot
                .start(req, &mut self.noise_music, &mut self.noise_writes);
        } else if self.noise_slot.active_effect != 0 {
            self.noise_slot
                .tick(&mut self.noise_music, &mut self.noise_writes);
        }
    }

    /// `$13F6B`: the engine reset `StartMusic` runs and that the stop-all
    /// (`$13F5E`) falls into. Clears every slot, takeover bit, slide/bend state
    /// and the low-time flag. `sFastMusic`, the song id, the wave-pattern
    /// pointer and `sSfx1Variant` are not in the ROM's list and survive here.
    pub fn reset_engine(&mut self) {
        let variant = self.wave_slot.variant;
        self.pulse_effect = PulseEffectState::default();
        self.wave_slot = WaveEffectState::default();
        self.wave_slot.variant = variant;
        self.noise_slot = NoiseEffectState::default();
        self.pulse_music.flags = PulseMusicFlags::default();
        self.pulse_music.vibrato_timer = 0;
        self.wave_music.flags = WaveMusicFlags::default();
        self.noise_music = NoiseMusicFlags::default();
        self.sound_mode = 0;
        self.low_time_flag = false;
        self.music_sequencer.drum_mode = 0;
        for ch in &mut self.music_sequencer.channels {
            ch.slide_state = 0;
            ch.slide_step = 0;
            ch.bend_flag = 0;
        }
    }

    /// `$11788`: a channel-4 note `$1F` starts noise effect 3 from inside the
    /// sequencer's own dispatch, which is why it is not a `sfx_request_2` write.
    fn start_music_drum(&mut self) {
        let has_drum_trigger = self.music_writes.contains(&MusicWrite::DrumTrigger);
        if has_drum_trigger && self.noise_slot.active_effect == 0 {
            self.noise_slot
                .start(3, &mut self.noise_music, &mut self.noise_writes);
        }
    }

    /// `$11510`: the `$9D` instrument command stores the channel-3 wave-pattern
    /// pointer, and the slot-1 stop (`$10E1D`) restores wave RAM through it.
    /// Keep the last pattern the music uploaded so that restore has the music's
    /// bytes instead of zeros.
    fn latch_ch3_pattern(&mut self) {
        for w in &self.music_writes {
            if let MusicWrite::Ch3WavePattern(pattern) = w {
                self.wave_music.wave_pattern = *pattern;
            }
        }
    }

    /// Complete per-frame sound update driver (_UpdateSound $10030).
    pub fn update_sound(&mut self) {
        self.update_count += 1;
        self.pulse_writes.clear();
        self.wave_writes.clear();
        self.noise_writes.clear();
        self.music_writes.clear();

        if self.sound_mode == 1 {
            self.sound_paused = true;
            self.clear_requests();
            return;
        }

        if self.sound_mode == 2 {
            self.sound_paused = false;
            self.clear_requests();
            return;
        }

        if self.sound_paused {
            self.clear_requests();
            return;
        }

        if self.music_request != 0 {
            let req = self.music_request;
            if req == 0xFF {
                // $112DD -> $13F5E: stop the song, then $13F6B's reset.
                self.music_sequencer.stop();
                self.reset_engine();
                self.active_song = 0;
            } else {
                // StartMusic runs $13F6B before it reads the song header.
                self.reset_engine();
                self.music_sequencer.start_song(req, &self.music_bank);
                self.active_song = self.music_sequencer.cur_song();
                // $10078 falls through to $10064, so the start frame still runs
                // the sequencer; the slots were just reset, so nothing masks.
                self.music_sequencer
                    .tick(&self.music_bank, [0, 0, 0], &mut self.music_writes);
                self.active_song = self.music_sequencer.cur_song();
                self.latch_ch3_pattern();
                self.start_music_drum();
            }
        } else {
            // Normal path: master volume derived from low_time_flag ($1004E..$1005A)
            self.master_volume = if self.low_time_flag {
                MASTER_VOLUME_LOW_TIME
            } else {
                MASTER_VOLUME_NORMAL
            };

            // Process SFX channels ($1005B..$10061)
            self.dispatch_slot_0();
            self.dispatch_slot_1();
            self.dispatch_slot_2();

            // Process Music channels ($10062..$11638)
            let sfx_active = [
                self.pulse_effect.active_effect,
                self.wave_slot.active_effect,
                self.noise_slot.active_effect,
            ];
            self.music_sequencer
                .tick(&self.music_bank, sfx_active, &mut self.music_writes);
            self.active_song = self.music_sequencer.cur_song();
            self.latch_ch3_pattern();
            self.start_music_drum();
        }

        // Common exit $10067: clears request bytes
        self.clear_requests();
    }
}
