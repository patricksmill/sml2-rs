//! Separate bank-12 castle visit ($467A) and boss-coin deposit ($4451).

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
/// `$499D`: the deposited coin's target y, indexed by `sCurLevel`.
pub const COIN_TARGET_Y: [u8; 25] = [
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
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SequenceOut {
    pub transition: Option<CastleTransition>,
    pub music: Option<u8>,
    pub sfx2: Option<u8>,
    /// The sequence handed control back (`sGameState` became `$14` or `$01E5`
    /// ran).
    pub done: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastleTransition {
    ReturnToMap { walked_out: bool },
    EnterLevel(u8),
    Reset,
}

pub const GOLDEN_COIN_OFFSETS: [usize; 6] = [5, 9, 14, 17, 23, 29];

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
    /// `sRowShiftCount`/`Index` (`$A2CA`/`$A2C9`).
    pub row_shift_count: u8,
    pub row_shift_index: u8,
    pub door_open_rows: u8,
    /// `sCastleCoinDone` (`$A2CC`): 0 or the six-coin tile `$74`.
    pub coin_done: u8,
    /// `sGoldenCoinCount` (`$A2CB`), recomputed after each menu handler.
    pub golden_count: u8,
    /// `sMarioWalkAnimTimer` (`$A222`).
    pub walk_anim: u8,
    /// Native OAM x/y ($FFC2/$FFC0); the source stages y before x.
    pub walk_x: u8,
    pub walk_y: u8,
    pub sprite_id: u8,
    pub sparkle: Option<(u8, u8, u8)>,
    /// `sCurLevel`.
    pub level: u8,
    /// `sLevelProgressFlags` (`$A27D`).
    pub progress_flags: u8,
    /// `sLevelTally` (`$A242`), BCD.
    pub level_tally: u8,
    pub tally_credited: bool,
    /// `sGoldenCoin1..6` as `$45B6` reads them.
    pub coins: [u8; 6],
    /// `sBGPalette`/`sOAMPalette1` (`$A27E`/`$A27F`).
    pub bgp: u8,
    pub obp1: u8,
    pub obp2: u8,
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
            door_open_rows: 0,
            coin_done: 0,
            golden_count: 0,
            walk_anim: 0,
            walk_x: 88,
            walk_y: 168,
            sprite_id: 0xBB,
            sparkle: None,
            level: 0,
            progress_flags: 0,
            level_tally: 0,
            tally_credited: false,
            coins: [0; 6],
            bgp: 0xE4,
            obp1: 0xD0,
            obp2: 0x38,
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
        self.walk_x = 88;
        self.walk_y = 168;
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
        // $0168: subtract from the whole little-endian timer with saturation.
        let fade = u16::from_le_bytes([self.fade_low, self.fade_high]).saturating_sub(1);
        [self.fade_low, self.fade_high] = fade.to_le_bytes();
        if self.row_shift_index != 0 {
            self.door_open_rows = 6 - self.row_shift_index;
            self.row_shift_index = 0;
        }
        self.sparkle = None;
        match self.state {
            STATE_REVEAL_A | STATE_REVEAL_D => {
                // `$45B6`: all six coins raise the castle's coin run.
                self.coin_done = if self.all_six_coins_flags() { 0x74 } else { 0 };
                self.bgp = 0xE4;
                self.obp1 = 0xD0;
                self.obp2 = 0x38;
                self.walk_x = 88;
                self.walk_y = 168;
                self.sprite_id = 0xBB;
                self.door_open_rows = if self.coin_done != 0 { 5 } else { 0 };
                self.menu_timer = 64;
                self.menu = 0;
                self.walk_anim = 0;
                if self.state == STATE_REVEAL_A {
                    out.music = Some(0xFF); // $30671: stop the music on $1B only
                }
                self.state += 1;
            }
            STATE_REVEAL_B => self.door_tick(input, &mut out),
            STATE_TALLY => {
                // `$441D`: the tally only counts the first clear of the level.
                if self.progress_flags & 0x80 == 0 && !self.tally_credited {
                    self.level_tally = bcd_add(self.level_tally, 1);
                }
                self.menu = 0;
                self.walk_anim = 0;
                self.walk_x = COIN_X.get(self.level as usize).copied().unwrap_or(0);
                self.walk_y = 192;
                self.sprite_id = 0xBF;
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
                if (self.fade_high == 0 && self.fade_low == 0) || input.keys_pressed == 0x08 {
                    out.done = true;
                    out.transition = Some(CastleTransition::Reset);
                }
            }
            _ => {}
        }

        self.golden_count = Self::count_coins(&self.coins);
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
                self.sparkle = Some((self.walk_x, self.walk_y, 0xC5 + ((input.frame & 4) >> 2)));
                self.walk_anim = (self.walk_anim + 8) % 0x50;
                self.sprite_id = 0xBF + (self.walk_anim >> 4);
                let target = COIN_TARGET_Y.get(self.level as usize).copied().unwrap_or(0);
                if self.walk_y == target {
                    out.music = Some(if self.golden_count == 5 { 26 } else { 15 });
                    self.menu = 1;
                    self.fade_high = 1;
                    self.fade_low = 0xFF;
                } else if input.frame & 1 == 0 {
                    self.walk_y = self.walk_y.wrapping_add(1);
                }
            }
            1 => {
                // `$44AC`: the reveal completes and the progress bit lands.
                if self.fade_high == 0 && self.fade_low == 0 {
                    self.obp1 = 0xD0;
                    self.progress_flags |= 0x80;
                    if let Some(index) = GOLDEN_COIN_OFFSETS
                        .iter()
                        .position(|&off| off == level_offset)
                    {
                        self.coins[index] = 0x80;
                    }
                    if let Some(byte) = progress.get_mut(level_offset) {
                        *byte = 0x80;
                    }
                    if self.golden_count == 5 {
                        self.menu = 2;
                        self.fade_low = 64;
                        self.row_shift_count = 5;
                    } else {
                        self.state = STATE_MAP;
                        out.done = true;
                        out.transition = Some(CastleTransition::ReturnToMap { walked_out: false });
                    }
                } else {
                    self.walk_y = COIN_TARGET_Y.get(self.level as usize).copied().unwrap_or(0);
                    self.sprite_id = COIN_SPRITE.get(self.level as usize).copied().unwrap_or(0);
                    self.obp1 = if input.frame & 0x1F < 4 { 0 } else { 0xD0 };
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
                    out.transition = Some(CastleTransition::ReturnToMap { walked_out: false });
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

// Bank 12 $4985 / $49B5. Offsets are indexed by sCurLevel (0..23).
const COIN_X: [u8; 24] = [
    0, 0, 0, 0, 0, 0x3B, 0, 0, 0, 0x69, 0, 0, 0, 0x75, 0, 0, 0x75, 0, 0, 0x3B, 0, 0, 0, 0x47,
];
const COIN_SPRITE: [u8; 24] = [
    0, 0, 0, 0, 0, 0xC4, 0, 0, 0, 0xDA, 0, 0, 0, 0xDB, 0, 0, 0xD8, 0, 0, 0xD7, 0, 0, 0, 0xD9,
];
#[path = "castle_sequence/door.rs"]
mod door;
#[cfg(test)]
#[path = "castle_sequence/tests.rs"]
mod tests;
