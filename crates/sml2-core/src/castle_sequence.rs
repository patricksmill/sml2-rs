//! The golden-coin castle sequence (`sGameState` `$1B`-`$21`).
//!
//! Bank 0's `$02B0` table dispatches `$1B`/`$1D` through `$2AD3`, `$1C`
//! through `$2ADE`, `$1E` through `$2AE9`, `$1F` through `$2AF4`, `$20`
//! through `$2A80` and `$21` through `$2A8B`; every stub switches to bank 12
//! and jumps. The handlers are mostly staging (castle tiles, the six coin
//! positions, the congratulations tilemap), so this module keeps the control
//! flow — the part later states branch on — and leaves the VRAM/OAM copies to
//! the renderer:
//!
//! - `$45B6` (`$1B`/`$1D`): clear `sCastleCoinDone`, write tile `$74` into the
//!   five four-byte runs when all six `sGoldenCoin1..6` flags have bit 7,
//!   BGP/OBP staging, `sMenuTimer = 64`, `sMenuState = 0`, state += 1.
//! - `$467A` (`$1C`): the `$446B` menu dispatcher.
//! - `$441D` (`$1E`): `sLevelTally` += 1 BCD unless `sLevelProgressFlags` bit
//!   7, tile-anim reset, fade `$C0`, `sMenuState = 0`, state += 1.
//! - `$4451` (`$1F`): the menu dispatcher again.
//! - `$477B` (`$20`): congratulations tilemap, fade `4`/`$FF`, state += 1.
//! - `$47A9` (`$21`): wait for the fade (or START) and fall into `$01E5`.
//!
//! The walk-in animation (`$453B`) moves `sOAMShadow`'s tile byte toward
//! `$499D[sCurLevel]`; here the caller reports `walk_done` when it arrives.

pub const STATE_REVEAL_A: u8 = 0x1B;
pub const STATE_REVEAL_B: u8 = 0x1C;
pub const STATE_REVEAL_D: u8 = 0x1D;
pub const STATE_TALLY: u8 = 0x1E;
pub const STATE_MENU: u8 = 0x1F;
pub const STATE_CONGRATS: u8 = 0x20;
pub const STATE_WAIT_START: u8 = 0x21;
/// The dispatcher's "leave the reveal, back to the map" state (`$14`).
pub const STATE_MAP: u8 = 0x14;

/// `$44F1`: `sCurLevel` -> the `sLevelProgress` byte the reveal marks. The
/// table is 25 bytes (levels 0-24).
/// `$499D`: the castle walk-in's target X, indexed by `sCurLevel`.
pub const WALK_TARGET_X: [u8; 25] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x79, 0x00, 0x00, 0x00, 0x46, 0x00, 0x00, 0x00, 0x5D, 0x00, 0x00,
    0x79, 0x00, 0x00, 0x5D, 0x00, 0x00, 0x00, 0x46, 0x00,
];

pub const LEVEL_PROGRESS_OFFSETS: [u8; 25] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x1D, 0x00, 0x00,
    0x17, 0x00, 0x00, 0x11, 0x00, 0x00, 0x00, 0x0E, 0x00,
];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SequenceInput {
    pub keys_pressed: u8,
    pub frame: u8,
    /// `$453B`'s walk-in reached `$499D[sCurLevel]`.
    pub walk_done: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SequenceOut {
    pub music: Option<u8>,
    pub sfx2: Option<u8>,
    /// The sequence handed control back (`sGameState` became `$14` or `$01E5`
    /// ran).
    pub done: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CastleSequence {
    /// `sGameState` (`$FF9B`).
    pub state: u8,
    /// `sMenuState` (`$A277`).
    pub menu: u8,
    /// `sMenuTimer` (`$A278`).
    pub menu_timer: u8,
    /// `sFadeTimerLow`/`High` (`$A266`/`$A267`).
    pub fade_low: u8,
    pub fade_high: u8,
    /// `sRowShiftCount`/`Index` (`$A2C9`/`$A2CA`).
    pub row_shift_count: u8,
    pub row_shift_index: u8,
    /// `sCastleCoinDone` (`$A2CB`): 0 or the six-coin tile `$74`.
    pub coin_done: u8,
    /// `sGoldenCoinCount` (`$A2CB` shared with the done byte in the ROM).
    pub golden_count: u8,
    /// `sMarioWalkAnimTimer` (`$A222`).
    pub walk_anim: u8,
    /// `$FFC0`/`$FFC2`: the walk-in's X and Y.
    pub walk_x: u8,
    pub walk_y: u8,
    /// `sCurLevel`.
    pub level: u8,
    /// `sLevelProgressFlags` (`$A27D`).
    pub progress_flags: u8,
    /// `sLevelTally` (`$A242`), BCD.
    pub level_tally: u8,
    /// `sGoldenCoin1..6` as `$45B6` reads them.
    pub coins: [u8; 6],
    /// `sBGPalette`/`sOAMPalette1` (`$A220`/`$A27F`).
    pub bgp: u8,
    pub obp1: u8,
}

impl Default for CastleSequence {
    fn default() -> Self {
        Self {
            state: STATE_REVEAL_A,
            menu: 0,
            menu_timer: 0,
            fade_low: 0,
            fade_high: 0,
            row_shift_count: 0,
            row_shift_index: 0,
            coin_done: 0,
            golden_count: 0,
            walk_anim: 0,
            walk_x: 16,
            walk_y: 40,
            level: 0,
            progress_flags: 0,
            level_tally: 0,
            coins: [0; 6],
            bgp: 0xE4,
            obp1: 0xD0,
        }
    }
}

impl CastleSequence {
    /// `$45B6`'s six-coin test: every `sGoldenCoin1..6` has bit 7 set.
    pub fn all_six_coins(coins: &[u8; 6]) -> bool {
        coins.iter().all(|&c| c & 0x80 != 0)
    }

    /// Count the set flags the way `$47BC` does.
    pub fn count_coins(coins: &[u8; 6]) -> u8 {
        coins.iter().filter(|&&c| c & 0x80 != 0).count() as u8
    }

    /// Run `$47BC`: count the flags and start the sequence.
    pub fn enter(&mut self, coins: &[u8; 6]) {
        self.coins = *coins;
        self.walk_x = 16; // `$4008`: the castle sprite starts at X 16, Y 40
        self.walk_y = 40;
        self.golden_count = Self::count_coins(coins);
        self.state = STATE_REVEAL_A;
    }

    /// One frame of the current state. `level_offset` is `$44F1[sCurLevel]`,
    /// the index into `sLevelProgress` the reveal completes.
    pub fn tick(
        &mut self,
        input: SequenceInput,
        level_offset: usize,
        progress: &mut [u8],
    ) -> SequenceOut {
        let mut out = SequenceOut::default();
        // The VBlank handler walks the fade timers down every frame.
        if self.fade_high != 0 {
            self.fade_high -= 1;
        } else if self.fade_low != 0 {
            self.fade_low -= 1;
        }
        match self.state {
            STATE_REVEAL_A | STATE_REVEAL_D => {
                // `$45B6`: all six coins raise the castle's coin run.
                self.coin_done = if self.all_six_coins_flags() { 0x74 } else { 0 };
                self.bgp = 0xE4;
                self.obp1 = 0xD0;
                self.menu_timer = 64;
                self.menu = 0;
                self.walk_anim = 0;
                if self.state == STATE_REVEAL_A {
                    out.music = Some(0xFF); // $30671: stop the music on $1B only
                }
                self.state += 1;
            }
            STATE_REVEAL_B => self.menu_tick(input, level_offset, progress, &mut out),
            STATE_TALLY => {
                // `$441D`: the tally only counts the first clear of the level.
                if self.progress_flags & 0x80 == 0 {
                    self.level_tally = bcd_add(self.level_tally, 1);
                }
                self.menu = 0;
                self.walk_anim = 0;
                self.fade_high = 0;
                self.fade_low = 0xC0;
                self.state += 1;
            }
            STATE_MENU => self.menu_tick(input, level_offset, progress, &mut out),
            STATE_CONGRATS => {
                // `$477B`: the congratulations tilemap, then the 4/$FF fade.
                self.fade_high = 4;
                self.fade_low = 0xFF;
                self.state += 1;
            }
            STATE_WAIT_START => {
                // `$47A9`: fade 0 or START both fall into `$01E5`.
                if (self.fade_high == 0 && self.fade_low == 0) || input.keys_pressed & 0x08 != 0 {
                    out.done = true;
                }
            }
            _ => {
                out.done = true;
            }
        }

        out
    }

    fn all_six_coins_flags(&self) -> bool {
        Self::all_six_coins(&self.coins)
    }

    /// `$446B`: the four sub-handlers.
    fn menu_tick(
        &mut self,
        input: SequenceInput,
        level_offset: usize,
        progress: &mut [u8],
        out: &mut SequenceOut,
    ) {
        match self.menu {
            0 => {
                // `$453B`: the walk-in; music follows the coin count.
                let target = WALK_TARGET_X
                    .get(self.level as usize)
                    .copied()
                    .unwrap_or(0x88);
                if self.walk_x != target && input.frame & 0x01 == 0 {
                    self.walk_x = self.walk_x.wrapping_add(1);
                }
                if input.walk_done || self.walk_x == target {
                    out.music = Some(if self.golden_count == 5 { 26 } else { 15 });
                    self.menu = 1;
                    self.fade_high = 1;
                    self.fade_low = 0xFF;
                }
            }
            1 => {
                // `$44AC`: the reveal completes and the progress bit lands.
                if self.fade_high == 0 && self.fade_low == 0 {
                    self.obp1 = 0xD0;
                    self.progress_flags |= 0x80;
                    if let Some(byte) = progress.get_mut(level_offset) {
                        *byte |= 0x80;
                    }
                    if self.golden_count == 5 {
                        self.menu = 2;
                        self.fade_low = 64;
                        self.row_shift_count = 5;
                    } else {
                        self.state = STATE_MAP;
                        out.done = true;
                    }
                }
            }
            2 => {
                // `$4477`: five 64-frame row shifts with SFX 2 = 10.
                if self.fade_low == 0 {
                    self.row_shift_index = self.row_shift_count;
                    if self.row_shift_count == 0 {
                        self.menu = 3;
                        self.fade_low = 0xFF;
                    } else {
                        self.row_shift_count -= 1;
                        self.fade_low = 64;
                        out.sfx2 = Some(10);
                    }
                }
            }
            _ => {
                // `$44A1`: the last fade hands back to the map.
                if self.fade_low == 0 {
                    self.state = STATE_MAP;
                    out.done = true;
                }
            }
        }
    }
}

/// `add`/`daa`: the BCD increment `$441D` uses.
fn bcd_add(value: u8, add: u8) -> u8 {
    let mut lo = (value & 0x0F) + (add & 0x0F);
    let mut hi = (value >> 4) + (add >> 4);
    if lo > 9 {
        lo -= 10;
        hi += 1;
    }
    if hi > 9 {
        hi -= 10;
    }
    (hi << 4) | lo
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags(set: usize) -> [u8; 6] {
        let mut coins = [0u8; 6];
        for coin in coins.iter_mut().take(set) {
            *coin = 0x80;
        }
        coins
    }

    #[test]
    fn the_six_coin_test_is_bit_seven_on_every_flag() {
        assert!(CastleSequence::all_six_coins(&flags(6)));
        assert!(!CastleSequence::all_six_coins(&flags(5)));
        assert_eq!(CastleSequence::count_coins(&flags(5)), 5);
    }

    /// `$45B6`: the reveal opens with the coin run set only when all six are
    /// collected, and `$1B` stops the music.
    #[test]
    fn the_reveal_checks_the_six_coins_and_advances() {
        let mut seq = CastleSequence::default();
        seq.enter(&flags(6));
        let mut progress = [0u8; 42];
        let out = seq.tick(SequenceInput::default(), 0, &mut progress);
        assert_eq!(seq.coin_done, 0x74);
        assert_eq!(seq.state, STATE_REVEAL_B);
        assert_eq!(seq.menu_timer, 64);
        assert_eq!(out.music, Some(0xFF));

        let mut seq = CastleSequence::default();
        seq.enter(&flags(4));
        let _ = seq.tick(SequenceInput::default(), 0, &mut progress);
        assert_eq!(seq.coin_done, 0, "four coins leave the run closed");
    }

    /// `$441D`: the tally counts once per level and the fade starts at `$C0`.
    #[test]
    fn the_tally_state_sets_the_reveal_fade() {
        let mut seq = CastleSequence::default();
        seq.state = STATE_TALLY;
        seq.progress_flags = 0;
        let mut progress = [0u8; 42];
        seq.tick(SequenceInput::default(), 0, &mut progress);
        assert_eq!(seq.state, STATE_MENU);
        assert_eq!(seq.fade_low, 0xC0);
        assert_eq!(seq.menu, 0);
    }

    /// `$453B` -> `$44AC`: arrival plays music 26 with five coins, then the
    /// progress bit lands and the sequence returns to the map.
    #[test]
    fn the_menu_walk_in_and_reveal_complete_the_progress_bit() {
        let mut seq = CastleSequence::default();
        seq.enter(&flags(5));
        seq.state = STATE_REVEAL_B;
        let mut progress = [0u8; 42];
        let out = seq.tick(
            SequenceInput {
                walk_done: true,
                ..Default::default()
            },
            0,
            &mut progress,
        );
        assert_eq!(out.music, Some(26), "$453B with five coins");
        assert_eq!(seq.menu, 1);
        assert_eq!((seq.fade_high, seq.fade_low), (1, 0xFF));

        // Burn the fade down, then the reveal marks the level.
        for _ in 0..300 {
            if seq.menu != 1 {
                break;
            }
            let _ = seq.tick(SequenceInput::default(), 0, &mut progress);
        }
        assert_eq!(seq.progress_flags & 0x80, 0x80);
        assert_eq!(seq.menu, 2, "five coins open the row shift");
        assert_eq!(seq.row_shift_count, 5);
    }

    /// `$44AC` marks `sLevelProgress[$44F1[sCurLevel]]` with bit 7.
    #[test]
    fn the_reveal_marks_the_level_progress_byte() {
        assert_eq!(LEVEL_PROGRESS_OFFSETS[9], 9);
        assert_eq!(LEVEL_PROGRESS_OFFSETS[23], 0x0E);
        let mut seq = CastleSequence::default();
        seq.enter(&flags(5));
        seq.state = STATE_REVEAL_B;
        seq.menu = 1;
        seq.fade_high = 0;
        seq.fade_low = 0;
        let mut progress = [0u8; 42];
        let _ = seq.tick(SequenceInput::default(), 9, &mut progress);
        assert_eq!(progress[9] & 0x80, 0x80);
        assert_eq!(seq.progress_flags & 0x80, 0x80);
    }

    /// `$453B`: the walk-in steps X once per even frame to `$499D[level]`.
    #[test]
    fn the_walk_in_reaches_the_level_target() {
        let mut seq = CastleSequence::default();
        seq.enter(&flags(5));
        seq.level = 5; // $499D[5] = $79
        seq.state = STATE_REVEAL_B;
        let mut progress = [0u8; 42];
        let mut frames = 0;
        for frame in 0..600u16 {
            let _ = seq.tick(
                SequenceInput {
                    frame: frame as u8,
                    ..Default::default()
                },
                5,
                &mut progress,
            );
            frames += 1;
            if seq.menu != 0 {
                break;
            }
        }
        assert_eq!(seq.walk_x, WALK_TARGET_X[5]);
        assert_eq!(seq.menu, 1);
        assert!(frames < 300, "about two frames per pixel");
    }

    /// `$4477`: five shifts, each waiting out a 64-frame fade with SFX 2 = 10,
    /// then `$44A1` hands back to the map.
    #[test]
    fn the_row_shift_runs_five_times_and_returns() {
        let mut seq = CastleSequence::default();
        seq.state = STATE_MENU;
        seq.menu = 2;
        seq.golden_count = 5;
        seq.row_shift_count = 5;
        let mut progress = [0u8; 42];
        let mut shifts = 0;
        for _ in 0..2000 {
            let out = seq.tick(SequenceInput::default(), 0, &mut progress);
            if out.sfx2 == Some(10) {
                shifts += 1;
            }
            if out.done {
                break;
            }
        }
        assert_eq!(shifts, 5);
        assert_eq!(seq.state, STATE_MAP);
    }

    /// `$21`: the fade expiring or START both end the sequence.
    #[test]
    fn the_start_wait_accepts_the_fade_or_start() {
        let mut seq = CastleSequence::default();
        seq.state = STATE_WAIT_START;
        seq.fade_low = 0x40;
        let mut progress = [0u8; 42];
        assert!(!seq.tick(SequenceInput::default(), 0, &mut progress).done);
        let out = seq.tick(
            SequenceInput {
                keys_pressed: 0x08,
                ..Default::default()
            },
            0,
            &mut progress,
        );
        assert!(out.done);
    }
}
