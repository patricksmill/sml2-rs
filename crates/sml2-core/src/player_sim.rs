//! Faithful port of the in-level Mario update loop (bank $00, `home.asm`
//! `$0F71` and its callees). Reconstructed from the disassembly:
//!
//! - `$13FE` per-frame resets (x-subpx, x-pushed, pipe align target)
//! - `$1BFF` water detection + the hazard probe (`$1C4E`/`$1C6D`: tiles
//!   `$50/$51` → `sMarioDead` 15, or `$80` when `sMarioScreenY >= $80`) and
//!   the exit-bell arm (`$1CBA`: the `$4A/$4B` tile → `sExitTileFlag`,
//!   `sMusicRequest = 3`, `$FF9B = 24` + a 192-frame fade)
//! - `$129F` landing check (gated floor probe `$1E3B`, `$12CE` landing)
//! - `$1413` wall push-out (`$1E7C`/`$1E8D` probes)
//! - `$14A6` ducking
//! - `$160A` jump (A press; bunny = A held; `$48..$BF` hvel + no UP
//!   downgrades super jumps to 48; swimming → 72)
//! - `$14FA` gravity + `$156B` vertical integration (`MarioSpeedTableV`
//!   family; ceiling probe `$1DFE`; jump-cut to 33 when A released)
//! - `$1701` horizontal acceleration (+8 in the held direction, caps
//!   $BF walk / $F3 run when B is held)
//! - `$16CC` friction (−4/frame toward $80 when grounded/swimming)
//! - `$181C` horizontal integration (`MarioSpeedTableH`; index = hvel +
//!   `$FF97 & 7` phase; wall probe revert; hvel = $80 on contact)
//! - `$0FBF` screen-edge wrap (uses the camera column $FFCA)
//! - `$14BF` walk-animation timer (`sMarioWalkAnimTimer`, +1/frame,
//!   +2 with B, wraps at $0C)
//! - `$134F`/`$1189`/`$10EC` pipe arming, entry and travel (the
//!   `$0F71`/`$0FBB` branch replaces the whole update while travelling)
//! - `$315C`/`$3186`/`$0F2A` death arc, fade and respawn
//! - the zero-lives game over: `$311A`'s `sub 1 / daa` borrow → `$27AF`
//!   (state `$05`, [`PlayerSim::game_over_enter`]) → `$287E` (state `$07`,
//!   [`PlayerSim::game_over_step`]: 255-frame fade, then START → `$00`)
//!
//! The engine's `ld b; add 16; ld b; add b; sub 16` position trick is a
//! signed add: values $F0..$FF subtract 16..1 (`sdelta` below).
//!
//! Omitted (flagged, not guessed): the goal bell's ring animation, fireball
//! paths, and the `$14` level-clear/map hand-off's banked body (`$3EF2B`: the
//! palette swap, the map-node advance and the save) — the port marks the
//! hand-off reach ([`PlayerSim::exit_completed`]) but keeps freezing. The same
//! applies to the game-over screen's banked body (`$27AF`: the `$4C92`
//! graphics and `$6492` tilemap loads, the `$2874` "GAME OVER" string, the
//! `$2934` save) — the port models its state writes and the `$07` START poll,
//! not the art. The `$532D` spin-jump pose is modelled
//! ([`PlayerSim::frame_index`]) but only unit-tested: its `$FF97 & $0C` index
//! needs bit 3, and captures record three bits of `$FF97`. Ported
//! since the first cut (this list used to claim otherwise): pipe
//! arming/entry/travel, water
//! detection (`$1BFF`), the `$129F` ledge launch, the death arc/fade/respawn
//! and the camera scroll deltas (in `camera.rs`, driven by the loop rather
//! than by this module); since wp7g the brick-bump system
//! (`$1993`/`$5960`/`$5997`), and since this cut the coin touch
//! (`$1D75`/`$1DA5`, called from the `$1E7C`/`$1E8D` side probes), the `$3451`
//! item consumer, the `$1BFF` exit-bell arm (`$1CBA`-`$1D01`), the `$126F`
//! spin jump with its `$1327` landing bounce (`$1993` off the floor probe,
//! no capture yet — unit-pinned), and the `$18` (`$3547`) exit-fade tail —
//! the BCD `sLevelTally` step with its `sLevelProgressFlags` bit-7 and
//! `sExitTileFlag` guards, run by
//! [`PlayerSim::exit_fade_step`] on the fade's 192nd frame.
//! The omitted state fields exist so later work slots in at the same points.

use crate::level::LevelLayout;

/// `MarioSpeedTableH` (ROM $19FF): horizontal speed by `hvel + phase`.
pub const SPEED_H: [u8; 256] = [
    0xFE, 0xFF, 0xFE, 0xFF, 0xFE, 0xFF, 0xFE, 0xFF, 0xFE, 0xFF, 0xFE, 0xFF, 0xFE, 0xFF, 0xFE, 0xFF,
    0xFE, 0xFF, 0xFE, 0xFF, 0xFE, 0xFF, 0xFE, 0xFF, 0xFE, 0xFF, 0xFE, 0xFF, 0xFE, 0xFF, 0xFE, 0xFF,
    0xFE, 0xFF, 0xFE, 0xFF, 0xFE, 0xFF, 0xFE, 0xFF, 0xFE, 0xFF, 0xFE, 0xFF, 0xFE, 0xFF, 0xFE, 0xFF,
    0xFE, 0xFF, 0xFF, 0xFE, 0xFF, 0xFF, 0xFE, 0xFF, 0xFE, 0xFF, 0xFF, 0xFF, 0xFE, 0xFF, 0xFF, 0xFF,
    0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0x00, 0xFF, 0xFF, 0xFF, 0x00, 0xFF, 0xFF, 0x00, 0xFF, 0xFF, 0x00, 0xFF, 0x00,
    0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0x00, 0xFF, 0x00, 0x00, 0xFF, 0x00,
    0xFF, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00,
    0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x01, 0x00, 0x01, 0x01, 0x00, 0x01, 0x00,
    0x01, 0x01, 0x01, 0x00, 0x01, 0x01, 0x01, 0x00, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x02, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x02, 0x01, 0x01, 0x01, 0x02, 0x01, 0x01, 0x01,
    0x02, 0x01, 0x01, 0x02, 0x01, 0x01, 0x02, 0x01, 0x02, 0x01, 0x02, 0x01, 0x02, 0x01, 0x02, 0x01,
    0x02, 0x01, 0x02, 0x01, 0x02, 0x01, 0x02, 0x01, 0x02, 0x01, 0x02, 0x01, 0x02, 0x01, 0x02, 0x01,
    0x02, 0x01, 0x02, 0x01, 0x02, 0x01, 0x02, 0x01, 0x02, 0x01, 0x02, 0x01, 0x02, 0x01, 0x02, 0x01,
];

/// `MarioSpeedTableV` (ROM $18AF).
pub const SPEED_V: [u8; 82] = [
    0x03, 0x03, 0x02, 0x02, 0x03, 0x02, 0x02, 0x02, 0x01, 0x02, 0x01, 0x01, 0x02, 0x01, 0x01, 0x01,
    0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0xFE, 0xFF,
    0xFE, 0xFE, 0xFE, 0xFD, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0xFD, 0xFD, 0xFD, 0xFE, 0xFD, 0xFD, 0xFD,
    0xFD, 0xFF, 0xFF, 0xFE, 0xFF, 0xFE, 0xFD, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0xFD, 0xFD, 0xFD, 0xFD,
    0xFD, 0xFD, 0xFD, 0xFD, 0xFD, 0xFD, 0xFD, 0xFD, 0xFD, 0xFD, 0xFD, 0xFD, 0x01, 0x01, 0x01, 0x00,
    0x01, 0x00,
];

/// `MarioSpeedTableVGoo` (ROM $18FB): swimming.
pub const SPEED_V_GOO: [u8; 82] = [
    0x01, 0x01, 0x01, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x00,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0x00, 0x00, 0xFF, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x01,
];

/// `MarioSpeedTableVWater` (ROM $1947): moon physics.
pub const SPEED_V_WATER: [u8; 82] = [
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x00,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xAF, 0xEA, 0x20, 0xA2,
    0xFA, 0x0D,
];

/// `MarioSpeedTableHWater` (ROM $1AFF): swimming / moon-grounded horizontal.
pub const SPEED_H_WATER: [u8; 256] = [
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0xFF, 0xFF, 0xFF, 0x00,
    0xFF, 0xFF, 0x00, 0xFF, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00,
    0xFF, 0x00, 0x00, 0xFF, 0x00, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00,
    0xFF, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00,
    0xFF, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00,
    0x01, 0x01, 0x00, 0x01, 0x01, 0x00, 0x01, 0x00, 0x01, 0x01, 0x01, 0x00, 0x01, 0x01, 0x01, 0x00,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
];

/// `MarioSpeedTableDead` ($31EF, home.asm): the death arc's per-vvel Y delta
/// (signed, 8-bit only — the engine's `add b` never carries into `$FFC1`).
pub const SPEED_V_DEAD: [u8; 96] = [
    0x03, 0x03, 0x03, 0x02, 0x02, 0x03, 0x02, 0x02, 0x02, 0x02, 0x01, 0x02, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFE, 0xFF, 0xFE, 0xFE,
    0xFE, 0xFE, 0xFD, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// `$5BB4` — the vertical offset (indexed by the bump timer) the type-2/3/6
/// draw adds to the block's screen Y before enqueueing the bump sprite.
const BOB_OFFSETS_5BB4: [i8; 8] = [0, 0, -2, -4, -6, -6, -4, -2];

/// `$5AE0` — the type-4 coin-pop offsets, 25 entries indexed by `timer >> 1`.
const COIN_OFFSETS_5AE0: [i8; 25] = [
    -12, -16, -20, -24, -27, -30, -33, -35, -36, -38, -40, -41, -41, -41, -40, -38, -37, -35, -33,
    -30, -27, -24, -20, -16, -12,
];

/// `$5A4A` — the type-1/5 (destroyed block) offsets, 40 entries indexed by the
/// timer; the draw stages two sprites, 101 at `x - timer` and 102 at `x + timer`.
const ITEM_POP_OFFSETS_5A4A: [i8; 40] = [
    -4, -8, -12, -16, -19, -22, -25, -27, -29, -30, -32, -33, -33, -33, -32, -30, -29, -27, -25,
    -22, -19, -16, -12, -8, -4, 0, 8, 12, 16, 21, 26, 31, 37, 43, 50, 57, 63, 70, 78, 80,
];

/// `$3439` — the fireball phase -> vertical step table, 24 entries. The phase
/// counts the arc down from `$17`: 3 px/frame early, 0 at the top (`$0B`), then
/// back up. Rising is `phase >= $0C`.
const FIRE_PHASE_STEP: [u8; 24] = [
    3, 3, 3, 3, 2, 3, 2, 2, 1, 2, 1, 0, 0, 1, 2, 1, 2, 2, 3, 2, 3, 3, 3, 3,
];

/// The projectile slots (`$A880`/`$A890`, stride `$10`, exactly 2): a global
/// array of its own, *not* the object slots the `B_*` constants index.
pub const F_ACTIVE: usize = 0;
pub const F_YLO: usize = 1;
pub const F_YHI: usize = 2;
pub const F_XLO: usize = 3;
pub const F_XHI: usize = 4;
pub const F_DIR: usize = 5;
pub const F_PHASE: usize = 6;
/// `$A880` + 2 * `$10` = `$A8A0` — both scans stop here.
pub const FIRE_SLOTS: usize = 2;

/// `inc a / daa` on a BCD byte (valid BCD in, valid BCD out): 09 -> 10,
/// 99 -> 00 with the carry out, which `$34AF` treats as the "cap at 99" case.
fn bcd_inc(v: u8) -> u8 {
    let mut lo = (v & 0x0F) + 1;
    let mut hi = v >> 4;
    if lo > 9 {
        lo -= 10;
        hi += 1;
    }
    if hi > 9 {
        return 0;
    }
    (hi << 4) | lo
}

/// `sub c / daa` on a BCD byte — `$27AF`'s golden-coin tally penalty. The ROM
/// subtracts the *count* of set golden coins (0-6, never a BCD digit above 9)
/// from `sLevelTally` and then `daa`s the result, so this is plain BCD
/// subtraction with a borrow that wraps, exactly as `daa` leaves it: 12 - 3 = 09,
/// and 05 - 6 = 99 rather than a negative.
fn bcd_sub(tally: u8, n: u8) -> u8 {
    let mut lo = (tally & 0x0F) as i16 - n as i16;
    let mut hi = (tally >> 4) as i16;
    if lo < 0 {
        lo += 10;
        hi -= 1;
    }
    if hi < 0 {
        hi += 10;
    }
    (((hi as u8) & 0x0F) << 4) | ((lo as u8) & 0x0F)
}

/// `add $50 / daa` on a BCD byte (`$350A`'s coin add). `$50`'s low nibble is
/// zero, so no half-carry correction is ever needed; the `+6` fires exactly
/// when the high nibble lands above 9, and that case also carries into the
/// next byte.
fn bcd_add_50(a: u8) -> (u8, bool) {
    let v = a.wrapping_add(0x50);
    if v >> 4 > 9 {
        (v.wrapping_add(0x60), true)
    } else {
        (v, false)
    }
}

/// The engine's `add b` / `sub 16` position trick: bytes $F0..$FF mean −16..−1.
#[inline]
pub fn sdelta(b: u8) -> i16 {
    if b < 0xF0 {
        b as i16
    } else {
        b as i16 - 0x100
    }
}

/// What one `$FF9B = $14` hand-off run did — see [`PlayerSim::map_handoff`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MapHandoffOutcome {
    /// Pass 1 ran: the post-clear walk cutscene's setup (`$3F4B1`/`$3F5A4`),
    /// modelled only as the `sGoalBell` clear it performs.
    pub cutscene: bool,
    /// `$2934` save calls the body made. The ROM calls it from `$3EEF0` (once,
    /// when the node has a progress slot) and again from `$3F03D` (once, for
    /// every node below `$1E`), so a plain level clear makes two.
    pub saves: u8,
    /// The node the hand-off ended on.
    pub node: u8,
    /// The zone screen the tail selected (`sMapScreenId`), when it selected one.
    ///
    /// The ROM's `$3F04D` bands and the castle arm both write `sMapScreenId` and
    /// then call that screen's loader (`$3D0FF`/`$3D208`/`$3D371`/`$3D49F`/
    /// `$3D61F`/`$3D79A`/`$3D6EB`/`$3D880`).
    pub screen: Option<u8>,
    /// The music track ID requested by the screen loader ($2AAA / sMusicRequest).
    pub music: Option<u8>,
}

/// Buttons held for one frame (plus a separate press edge for A/Mario).
#[derive(Debug, Clone, Copy, Default)]
pub struct PlayerKeys {
    pub right: bool,
    pub left: bool,
    pub up: bool,
    pub down: bool,
    /// Held state — bunny jumps use the hold, everything else the press.
    pub a: bool,
    pub b: bool,
    /// START / SELECT held. Neither is a gameplay input: the in-level code
    /// reads them only through `hKeysPressed` in `$273E`/`$2756` (START enters
    /// and leaves the pause state; SELECT with `sLevelProgressFlags` bit 7
    /// set abandons the level). They matter to the `$307D` crouch pan too,
    /// whose `cp $82`/`cp $42` are whole-byte tests — holding either cancels
    /// a pan.
    pub start: bool,
    pub select: bool,
}

/// One frame of simulated player state (the `$FFC0`-family + speed indexes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerState {
    pub x: u16,
    pub y: u16,
    pub hvel: u8,
    pub vvel: u8,
    pub on_ground: bool,
    pub swim: bool,
}

#[derive(Debug, Clone)]
pub struct PlayerSim {
    pub x: u16,
    pub y: u16,
    pub hvel: u8,
    pub vvel: u8,
    pub on_ground: bool,
    pub in_air: bool,
    pub dir_right: bool,
    pub ducking: bool,
    pub swim: bool,
    pub moon: u8,
    /// Entry-frame snapshot of `$A2B3`; CameraSim owns the draw-time countdown.
    pub pan_timer: u8,
    /// `$A254/$A255/$A24B/$A53F`: level countdown and its warning latch.
    pub level_timer: crate::level_timer::LevelTimer,
    /// `$A469`: acknowledged playing song, owned by the sound driver.
    pub current_song: u8,
    /// `$A292`: the level's music selection.
    pub level_song: u8,
    /// `sMoonFloatTimer` ($A2B2) — the bit-3 timed float's budget: 50 armed on
    /// the ground (`$104A`), 0 when A is released (`$1050`), cleared by a
    /// ceiling bonk (`$15DA`).
    pub moon_float_timer: u8,
    /// `sMoonJumpPoseTimer` ($A285) — the bit-1 moon-zone re-launch's pose
    /// decay: 8 on the launch (`$1066`), read down by bank 1's `$5354`/`$53CF`.
    pub moon_jump_pose_timer: u8,
    /// `sFlutterTimer` ($A286) — bunny flutter decay timer.
    pub flutter: u8,
    /// `sFlutterArmed` ($A287) — bunny flutter armed latch.
    pub flutter_armed: bool,
    /// `sFlutterCharge` ($A288) — charge counter for repeated flutter while holding A.
    pub flutter_charge: u8,
    pub powerup: u8,
    pub goal_bell: bool,
    pub on_ground_obj: bool,
    pub jump_flag: bool,
    pub no_jump_cut: bool,
    pub x_pushed: bool,
    pub x_sub: u8,
    pub pipe_align_target: u8,
    /// `sPipeTravelDirection` ($A221): 0 none, 1 armed-down, 2 armed-up,
    /// 3 armed-side-right, 4 armed-side-left, $10/$20/$30/$40 = travelling.
    pub pipe_dir: u8,
    /// `sSidePushDir` ($A28A): 1 pushed right, $FF pushed left.
    pub side_push_dir: u8,
    /// `sSfxRequest0` — fidelity only (not replay-compared).
    pub sfx0: u8,
    /// `sSfxRequest1` ($A470) — fidelity only. `$3451` writes 20 for the
    /// three powerup items (ids 1/2/3) and the moon (id 8); the exit-bell
    /// arm clears it alongside sfx0.
    pub sfx1: u8,
    /// `sSfxRequest2` ($A478) — fidelity only. The big-Mario block
    /// destruction arms (`$19B0`/`$19C1`) request 1 (the break sound); the
    /// exit-bell arm clears it alongside sfx0/sfx1.
    pub sfx2: u8,
    /// `sMarioDead` ($A224): 0 alive, 15 just hit, $80 instant, $FF the arc.
    pub dead: u8,
    /// `sInvincibilityTimer` ($A217): set on a powerup loss, flashes the
    /// sprite (+32 frame offset while bit 2 is set).
    pub invincibility: u8,
    /// `$FF9B = 9` — the death fade. The frame loop leaves the level, so the
    /// whole sim (player, camera, objects) freezes here.
    pub dead_fade: bool,
    /// `$FF9B = 24` — the exit/level-clear phase, set by the `$1CBA` bell arm.
    /// State `$18` (`$3547`) replaces the in-level loop, so — like the death
    /// fade — the whole sim freezes while this is set. `exit_fade_step` models
    /// VBlank's `sFadeTimer` decrement (which keeps running in state `$18`),
    /// and its final call runs the `$3547` tail: the BCD level-tally step and
    /// the hand-off to `$14`.
    pub exit_phase: bool,
    /// `sBossClearFlag` (`$A2E3`): armed by the boss hit pass (`$92C4`) when a
    /// fight's hp reaches zero. Its sole ROM read suppresses countdown writes
    /// (`$5D6E..$5D77`). The `$5E12` continuation checks boss fight phase;
    /// the clear flag itself does not request a level exit or coin reward.
    pub boss_clear: bool,
    /// `$FF9B = 20` — `$14`, the level-clear/map hand-off the `$3547` tail
    /// enters once the exit fade's last frame has run. The hand-off's banked
    /// body is modelled by [`PlayerSim::map_handoff`] as far as the port can
    /// observe it (the map-node rewrite, the progress bit and the move to the
    /// map state); the overworld it hands off *to* is not ported, so the port
    /// keeps freezing on `exit_phase` afterwards. This flag marks that the tail
    /// ran.
    pub exit_completed: bool,
    /// `sMapNode` (`$A840`) — the overworld node the player is on. The `$14`
    /// hand-off rewrites it (see [`PlayerSim::map_handoff`]) and the map state
    /// reads it to choose the next level, so it is progress state and survives
    /// a level reload. A level entered from the map starts with the node the
    /// map selected; a plain level's node is 0, which is exactly the case the
    /// hand-off's tail keys on.
    pub map_node: u8,
    /// `sLevelProgress` (`$A848`) — the **persisted** per-level flags, indexed
    /// by a node's `$650E` progress slot (`crate::map::MapNodes`): bit 7 = the
    /// level has been cleared, bit 3 = it was entered by the secret route. The
    /// hand-off writes these; the map copies the entry into
    /// `sLevelProgressFlags` (`$A27D`, [`PlayerSim::level_progress_flags`]) when
    /// a level is entered, and that working copy is what the `$3547` tally guard
    /// and the `$2756` SELECT abandon read. 42 bytes covers the highest slot in
    /// the ROM's `$650E` table ($29).
    pub level_progress: [u8; 42],
    /// `sGoldenCoinFlags` (`$A2B5`): `$27AF`'s fold of the six golden-coin bytes,
    /// one bit each — bit 5 = coin 1, bit 4 = coin 2, bit 3 = coin 6, bit 2 =
    /// coin 5, bit 1 = coin 4, bit 0 = coin 3. The `$30884` icon draw (state
    /// `$07`, and the castle screen) tests bit 5 alone, which is why the fold
    /// happens *before* `$27AF` clears the six bytes.
    pub golden_coin_flags: u8,
    /// The port's marker that the `$FF9B = $14` body has run. The ROM needs no
    /// flag: the body ends by setting `$FF9B = 12` (the map), so it runs on
    /// exactly one frame. Without an overworld to enter, the port records it
    /// here instead — cleared by a level reload like the other phase flags.
    pub map_handoff_done: bool,
    /// `$FF9B = 5` — the GAME OVER screen (`$27AF`), entered from the death
    /// fade's borrow (`$311A`'s `sub 1 / daa` on `sLives` with carry set,
    /// i.e. `sLives == 0`). The bank-12 body is not modelled (the `$4C92`
    /// graphics load, the `$6492` tilemap, the `$2874` "GAME OVER" string);
    /// the port keeps the state fields `$27AF` writes
    /// ([`PlayerSim::game_over_enter`]) and the `$07` poll
    /// ([`PlayerSim::game_over_step`]).
    pub game_over: bool,
    /// `$FF9B = 8` — the pause state (`$2756`). The ROM replaces the whole
    /// state-`$04` body with this handler, so the port freezes the frame the
    /// same way it does on a fade (player, camera and object engine all stop)
    /// while the frame counter keeps ticking for
    /// [`PlayerSim::pause_flash`].
    pub paused: bool,
    /// `sLevelProgressFlags` ($A27D): bit 7 = the stage was already cleared
    /// (the map sets it when the node completes), so `$3547` skips the tally
    /// on a replay. Nothing in-level writes it.
    pub level_progress_flags: u8,
    /// `sLives` ($A22C, BCD) — `$311A` decrements it when the fade ends.
    pub lives: u8,
    /// `sLevelGfxIndex` (`$A20B`): the tileset index the last `$6x` selected.
    pub level_gfx_index: u8,
    /// `sFadeTimer` ($A266/$A267) — `$3186` sets it to 192 when the fade
    /// starts; VBlank decrements it every frame (saturating), and the fade's
    /// last frame runs the level-end tail.
    pub fade_timer: u16,
    pub walk_timer: u8,
    /// `$FF97 & 7` — the global sub-frame phase (drives the speed dither).
    pub phase: u8,
    /// `$FF97` — the once-per-frame global counter itself. The pause flash
    /// reads bit 4 of it every frame, so it is a 32-frame cycle off the
    /// counter rather than a timer with a reload value.
    pub frame_counter: u8,
    /// `sCameraX` low byte **as of the frame's start** — the `$0FBF` screen
    /// clamp reads `$FFCA` during `$0F71`, before that frame's `$0B2A` camera
    /// step, so the caller latches this before stepping the camera (both
    /// frame loops do). Latching the post-step byte shifts the clamp one
    /// frame ahead: on an autoscroll level it pushes Mario +1 early on every
    /// frame the camera steps (traj19, `autoscroll_clamp_x_divergence`).
    pub camx: u16,
    /// `sCameraY` low byte, same frame-start timing as [`Self::camx`] —
    /// `$1BFF`'s hazard arm reads `sMarioScreenY`, which the draw step
    /// computed from the previous frame's camera.
    pub camy: u16,
    /// Mirror of sCollisionTile (last sampled tile).
    pub collision_tile: u8,
    /// The ROM's shared probe staging, `$FFB7`/`$FFB9` (+ the high bytes in
    /// `$FFB8`/`$FFBA`): the point `$1EFA` reads a tile from, written by
    /// *both* probes and read by `$5960`, which stores a bump slot's cell from
    /// it. `$1DFE` (the ceiling probe) leaves `(x+16, y+28|18)`, `$1E3B` (the
    /// floor probe) `(x+12|20, y+44)` — whichever ran last wins, which is what
    /// makes a spinning Mario's *landing* bonk (`$1327`, the floor point) and
    /// his *ceiling* bonk (`$15F4`, the ceiling point) both store the block
    /// they actually hit.
    pub probe_point: (u16, u16),
    /// `sPendingItemId` ($A26C) — staged by the object pass (`$2DC6`) and
    /// consumed by `$3451` at the next frame's update tail.
    pub pending_item: u8,
    /// The one-shot `sStompChainCount` ($A279) reset `$3451` id 4 owes when
    /// the previous star timer sits at its exhausted marker: the counter
    /// itself lives in the object engine (`objects::ObjectSim::stomp_chain`,
    /// bumped by the `$2EAA` star defeat), so the consumer only raises this
    /// flag and the runtime applies it before the frame's object pass.
    pub stomp_chain_reset: bool,
    /// `sSpinJumpBounce` ($A220) — `$1993` clears it at the top of every
    /// ceiling bonk and sets `$FF` on the bump classes; nothing else in the
    /// in-level loop writes it.
    pub spin_jump_bounce: u8,
    /// `sSpinJump` ($A268): `$FF` while the down-in-air spin jump is
    /// engaged. `$126F` (the port's `step`, between the ducking and jump
    /// stages) sets it while DOWN is held, `sCurPowerup` is odd,
    /// `sMarioOnGround` is clear and there is no moon physics — and clears
    /// it otherwise. Read by the `$1327` landing arm and the object
    /// engine's stomp dispatch (`$85C4`).
    pub spin_jump: u8,
    /// `sObjectBumpSpecial` ($A256) — `$2F72` (the tail of the slot alloc)
    /// clears it, then sets it to `$FF` when the below-hit pass lands on a
    /// type-`$80` pose; `$19D2` reads it to decide whether the `$01` bonk
    /// still owes the coin. The object side computes the flag
    /// (`ObjectSim::below_bonk`) and the frame loop hands it back through
    /// [`PlayerSim::resolve_bump_coin`], which is where the coin is decided.
    pub object_bump_special: u8,
    /// `$2F72`'s bonk point — the ceiling probe's world low bytes
    /// (`$FFB7`/`$FFB9`), armed by the bump alloc (`$5960`) and consumed by
    /// the object side's below-hit pass before the same frame's object
    /// engine. `Some` only between the alloc and that hand-off.
    pub below_bonk: Option<(u8, u8)>,
    /// `$19D2`'s deferred tail: the `$01` bonk's coin and its type-4 slot
    /// wait on `sObjectBumpSpecial`, which the object side's `$2F72` decides
    /// (a type-`$80` pose in the bonk window withholds them). The frame loop
    /// resolves it via [`PlayerSim::resolve_bump_coin`] right after `step()`.
    pub bump_coin_pending: bool,
    /// Level-map cells a rewrite has written into WRAM's copy (`$5CF1` from a
    /// brick bump, `$1DA5`/`$5CF2` from the coin touch), so every probe reads
    /// the new tile afterwards: a bumped `$01` block reads `$07` and stops
    /// being bumpable, a collected `$48` coin reads `$60` and stops
    /// collecting. Sparse: one entry per touched cell. The engine's separate
    /// BG-map (VRAM) writes — the `$FF`/`$F4-$FB`/`$F8-$FB` 2x2 art and
    /// `$1DA5`'s own `$FF` pair — are not modelled as collision; the renderer
    /// *does* draw the overlay (see `render_view`).
    pub layout_writes: Vec<(u32, u8)>,
    /// `sBlockAnimSlots` ($AA80): 8 slots x $10 for the brick-bump animations.
    /// `+0` type, `+1` timer, `+2/+3` Y, `+4/+5` X; `$5960` allocates, `$5997`
    /// ticks them. The other 10 bytes are only ever written by the level-load
    /// clear (`$049C`) and a freed slot keeps its old contents (only the type
    /// byte is zeroed), so a byte-exact capture check needs the whole 16.
    pub block_slots: [[u8; 16]; 8],
    /// The frame `$2C82`/`$5997` drew into `$FFC6` this frame (the *last*
    /// drawing slot wins — a `$01` bonk allocates a type 3 and then a type 4
    /// slot, so the coin-pop cycle is what the console shows). `None` = no
    /// bump drew, i.e. `$FFC6` keeps the frame selection's value.
    pub block_frame: Option<u8>,
    /// The bump's *own* sprite, staged this frame by the same draw routines
    /// (`$5297`/`$527C`): the engine writes `$FFC4`/`$FFC5`/`$FFC6` for the
    /// block's screen coords and enqueues it, so a bump is an *extra* OAM
    /// sprite — Mario's own sprite keeps his pose. World-space `(x, y, frame)`
    /// in this port's sprite convention (`$draw_cells` adds `world - cam +
    /// 96/98`, cancelled here so the renderer lands on the engine's `$FFC4`/
    /// `$FFC5`). Types 1/5 push two entries (the `$5A07` pair).
    pub block_draw: Vec<(u16, u16, u8)>,
    /// The two projectile slots (`$A880`/`$A890`, stride `$10`; see
    /// [`F_ACTIVE`]): `$32C1` spawns them, `$331C` moves them, `$324F`
    /// draws/culls/object-passes them, and both scans stop at `$A0`.
    pub fireballs: [[u8; 16]; FIRE_SLOTS],
    /// `sMarioThrowTimer` ($A25A) — the spawn sets 8 and only the Mario *draw*
    /// reads it (the throw pose). It gates no firing: the press edge does.
    pub throw_timer: u8,
    /// `$3261`'s staged screen pair `(sy, sx)` per live slot. The pass's
    /// `$2FED` object scan lives in the object engine, so the frame loop reads
    /// this pair, runs `ObjectSim::fire_hit` and hands the verdict back
    /// through [`Self::resolve_fire_hit`].
    pub fire_scan: [Option<(u8, u8)>; FIRE_SLOTS],
    /// `$2F72` armed by a fireball's `$06` break instead of by the player's
    /// ceiling bonk — the frame loop runs the below-hit pass for it too, at
    /// the ball's own cell bytes.
    pub fire_below_bonk: Option<(u8, u8)>,
    /// `$3261`'s own sprite staging, one entry per live ball: `$2CF4` expands
    /// the mapping `$B0`/`$B4` + `($FF97 & 6) >> 1` (four flicker frames each
    /// way, rDiv bits 1-2) at the ball's staged pair. `(x, y, mapping)` in the
    /// same renderer convention as [`Self::block_draw`]: `draw_cells` adds
    /// `world - cam + 96/98`, while `$3261` stages `X - camX + 80` /
    /// `Y - camY + 70`, hence the `-16`/`-28` anchors.
    pub fire_draw: Vec<(u16, u16, u8)>,
    /// `sCoinLow`/`sCoinHigh` ($A262/$A263, BCD).
    pub coins_low: u8,
    pub coins_high: u8,
    /// `sSecretRouteFlag` ($A2A0) — item id 7 sets it to `$FF`.
    pub secret_route: u8,
    /// `sStarTimerHigh`/`sStarTimerLow` ($A271/$A272) — item id 4 sets 4/64.
    pub star_high: u8,
    pub star_low: u8,
    /// `sLowTimeFlag` ($A53F) — the low-time warning latch the exit bell
    /// clears. Fidelity only (the time warning itself is not modelled).
    pub low_time_flag: u8,
    /// `sExitTileFlag` ($A2B4) — `$FF` when the exit was taken through the
    /// `$4B` ("route already cleared") bell tile, else 0. The `$3547`
    /// level-tally guard reads it back.
    pub exit_tile_flag: u8,
    /// `sLevelTally` ($A2C5, BCD) — the per-file stage tally drawn on the
    /// file-select screen. `$3547`'s post-bell step adds 1 (`add 1 / daa`)
    /// unless `sLevelProgressFlags` bit 7 or `sExitTileFlag` guards it.
    pub level_tally: u8,
    /// `sMusicRequest` ($A468) — the exit bell requests track 3 (the level
    /// clear fanfare). Fidelity only (audio is not modelled).
    pub music_request: u8,
    /// `sTileAnimConfig` ($A299) — the exit bell clears it. Fidelity only:
    /// the port's tile animation comes from the level manifest.
    pub tile_anim_config: u8,
}

impl Default for PlayerSim {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerSim {
    pub fn new() -> Self {
        // `$0F2A` level init: hvel = $80, vvel = 24, facing right.
        Self {
            x: 0,
            y: 0,
            hvel: 0x80,
            vvel: 24,
            on_ground: true,
            in_air: false,
            dir_right: true,
            ducking: false,
            swim: false,
            moon: 0,
            pan_timer: 0,
            level_timer: crate::level_timer::LevelTimer::new(0),
            current_song: 0,
            level_song: 0,
            moon_float_timer: 0,
            moon_jump_pose_timer: 0,
            flutter: 0,
            flutter_armed: false,
            flutter_charge: 0,
            powerup: 1,
            goal_bell: false,
            on_ground_obj: false,
            jump_flag: false,
            no_jump_cut: false,
            x_pushed: false,
            x_sub: 0,
            pipe_align_target: 0,
            pipe_dir: 0,
            side_push_dir: 0,
            sfx0: 0,
            sfx1: 0,
            sfx2: 0,
            dead: 0,
            invincibility: 0,
            dead_fade: false,
            exit_phase: false,
            boss_clear: false,
            exit_completed: false,
            map_node: 0,
            level_progress: [0; 42],
            golden_coin_flags: 0,
            level_gfx_index: 0,
            map_handoff_done: false,
            game_over: false,
            paused: false,
            level_progress_flags: 0,
            lives: 5,
            fade_timer: 0,
            walk_timer: 0,
            phase: 0,
            frame_counter: 0,
            camx: 0,
            camy: 0,
            collision_tile: 0,
            probe_point: (0, 0),
            pending_item: 0,
            stomp_chain_reset: false,
            spin_jump_bounce: 0,
            spin_jump: 0,
            object_bump_special: 0,
            below_bonk: None,
            bump_coin_pending: false,
            layout_writes: Vec::new(),
            block_slots: [[0; 16]; 8],
            block_frame: None,
            block_draw: Vec::new(),
            fireballs: [[0; 16]; FIRE_SLOTS],
            throw_timer: 0,
            fire_scan: [None; FIRE_SLOTS],
            fire_below_bonk: None,
            fire_draw: Vec::new(),
            coins_low: 0,
            coins_high: 0,
            secret_route: 0,
            star_high: 0,
            star_low: 0,
            low_time_flag: 0,
            exit_tile_flag: 0,
            level_tally: 0,
            music_request: 0,
            tile_anim_config: 0,
        }
    }

    /// Collision sample: `$1EFA` = `$B000 + ((y-16)>>4)*$100 + ((x-8)>>4)`.
    /// Cells a rewrite wrote (`layout_writes`: a brick bump's `$5CF1` or the
    /// coin touch's `$1DA5`/`$5CF2`) win over the extracted map — the engine
    /// edits the same WRAM copy the probes read.
    fn tile(&self, layout: &LevelLayout, px: u16, py: u16) -> u8 {
        let row = ((py.wrapping_sub(16)) >> 4) & 0xFF;
        let col = ((px.wrapping_sub(8)) >> 4) & 0xFF;
        if (row as usize) < 48 {
            let key = (row as u32) * 256 + col as u32;
            if let Some((_, v)) = self.layout_writes.iter().find(|(k, _)| *k == key) {
                return *v;
            }
            layout.cells[(row as usize) * 256 + col as usize]
        } else {
            0 // out of the 48-row map: engine would read adjacent WRAM; not expected
        }
    }

    /// `$1E3B` gated floor probe. Returns the standable tile (< $48).
    ///
    /// It stages `$FFB7`/`$FFB9` before each of its two reads — `(x+12, y+44)`
    /// first, then `(x+20, y+44)` only when the first found no floor — so after
    /// it returns the staging is the point that last probed, and the `$1993`
    /// a spinning landing runs straight afterwards (`$1327`) allocates the bump
    /// slot at the cell that probe looked at. The `(y-4) & $F` early return
    /// stages nothing, exactly like the ROM's `cp $05 / jr nc`.
    fn floor_probe(&mut self, layout: &LevelLayout) -> Option<u8> {
        if (self.y.wrapping_sub(4) & 0x0F) >= 5 {
            return None;
        }
        self.probe_point = (self.x.wrapping_add(12), self.y.wrapping_add(44));
        let t1 = self.tile(layout, self.probe_point.0, self.probe_point.1);
        if t1 < 0x48 {
            return Some(t1);
        }
        self.probe_point = (self.x.wrapping_add(20), self.y.wrapping_add(44));
        let t2 = self.tile(layout, self.probe_point.0, self.probe_point.1);
        if t2 < 0x48 {
            return Some(t2);
        }
        None
    }

    /// `$1DFE` ceiling probe at (x+16, y + 28 small / 18 big).
    fn ceiling_hit(&mut self, layout: &LevelLayout) -> bool {
        self.ceiling_tile(layout).is_some()
    }

    /// `$1DFE` with the tile — `$1EFA` writes every probed tile into
    /// `sCollisionTile`, and the bonk path (`$15F4`) reads it back. The probe
    /// point itself stays in `$FFB7`/`$FFB9` and is what a bump slot stores.
    fn ceiling_tile(&mut self, layout: &LevelLayout) -> Option<u8> {
        let dy = if self.powerup == 0 { 28 } else { 18 };
        let px = self.x.wrapping_add(16);
        let py = self.y.wrapping_add(dy);
        self.probe_point = (px, py);
        let t = self.tile(layout, px, py);
        if t == 0x7E || t < 0x38 {
            Some(t)
        } else {
            None
        }
    }

    /// `$1E7C` (left, x+11) / `$1E8D` (right, x+20) wall probes at y+20/+32/+40.
    /// Returns the first hit tile — the engine leaves it in `sCollisionTile`,
    /// which the side-pipe arming (`$143D`/`$147F`) then inspects. Every probed
    /// tile at or above `$38` goes through `$1D75` — the *coin touch* — before
    /// the next probe point, so walking into a coin collects it here (once per
    /// probe call that reads one, in the frame's own call order).
    fn wall_tile(&mut self, layout: &LevelLayout, px: u16) -> Option<u8> {
        if self.powerup != 0 && !self.ducking {
            let py = self.y.wrapping_add(20);
            let t = self.tile(layout, px, py);
            if t < 0x38 {
                return Some(t);
            }
            self.coin_touch(t, px, py);
        }
        let py = self.y.wrapping_add(32);
        let t = self.tile(layout, px, py);
        if t < 0x38 {
            return Some(t);
        }
        self.coin_touch(t, px, py);
        let py = self.y.wrapping_add(40);
        let t = self.tile(layout, px, py);
        if t < 0x38 {
            return Some(t);
        }
        self.coin_touch(t, px, py);
        None
    }

    fn wall_hit(&mut self, layout: &LevelLayout, px: u16) -> bool {
        self.wall_tile(layout, px).is_some()
    }

    fn left_wall(&mut self, layout: &LevelLayout) -> bool {
        self.wall_hit(layout, self.x.wrapping_add(11))
    }

    fn right_wall(&mut self, layout: &LevelLayout) -> bool {
        self.wall_hit(layout, self.x.wrapping_add(20))
    }

    /// `$1D75`: the coin touch, called by the side probes for every tile they
    /// read at or above `$38`. `sCollisionTile & $FE == $48` selects the pair
    /// (`$48`/`$49` are both class `coin`); the routine requests sfx 7, runs
    /// `$1DA5` — which rewrites the level map at the probe's *own* cell — and
    /// credits the BCD counter.
    fn coin_touch(&mut self, tile: u8, px: u16, py: u16) {
        if tile & 0xFE != 0x48 {
            return;
        }
        self.sfx0 = 7;
        // $1DA5 re-reads `sCollisionTile` and branches on `cp $48` exactly:
        // `$48` → 96 (`$60`, empty air), anything else through the `& $FE`
        // door ($49) → 127 (`$7F`). Both are >= $38, so the cell stays
        // passable; the write lands in WRAM's copy of the level map, which
        // every later probe (and the renderer) reads.
        let replacement = if tile == 0x48 { 0x60 } else { 0x7F };
        let row = ((py.wrapping_sub(16)) >> 4) & 0xFF;
        let col = ((px.wrapping_sub(8)) >> 4) & 0xFF;
        self.layout_write(row as u8, col as u8, replacement);
        // $1D75's tail: `inc a / daa` + `adc 0`, clamped at `$0999`.
        self.coin_credit();
    }

    /// `$1BFF`: fresh water detection at (x+16, y+28) (& $FC == $4C).
    fn detect_swim(&self, layout: &LevelLayout) -> bool {
        let t = self.tile(layout, self.x.wrapping_add(16), self.y.wrapping_add(28));
        t & 0xFC == 0x4C
    }

    /// `$1BFF` hazard arm (`$1C39`/`$1C4E`/`$1C6D`): a `$50/$51` tile
    /// (`tile & $FE == $50`) at `(x+16, y+22)` for big Mario on his feet —
    /// falling through to the `(x+16, y+28)` probe when that misses — else at
    /// `(x+16, y+28)` alone. Returns `(entered, dead)`: whether the arm was
    /// taken (it `jp`s the tail, so the water and exit-bell arms are skipped)
    /// and the `sMarioDead` byte it writes — `15`, or `$80` once the draw's
    /// screen Y `sMarioScreenY = y - camY + 98` reaches `$80`. `$1C6D` is the
    /// fall-through of both the kill arm and the invincibility/star guard, so
    /// the `$80` overwrite fires even while invincible. The caller lands the
    /// byte just before the `$319F` dispatch (same frame; see `step`).
    fn hazard_probe(&self, layout: &LevelLayout) -> (bool, Option<u8>) {
        let probe_y = if self.powerup != 0 && !self.ducking {
            22
        } else {
            28
        };
        let t = self.tile(
            layout,
            self.x.wrapping_add(16),
            self.y.wrapping_add(probe_y),
        );
        let hit = if t & 0xFE == 0x50 {
            true
        } else if probe_y == 22 {
            // $1C39: the big-Mario arm's own fall-through y+28 probe.
            self.tile(layout, self.x.wrapping_add(16), self.y.wrapping_add(28)) & 0xFE == 0x50
        } else {
            false
        };
        if !hit {
            return (false, None);
        }
        let screen_y = (self.y as u8)
            .wrapping_sub(self.camy as u8)
            .wrapping_add(98);
        if screen_y >= 0x80 {
            return (true, Some(0x80));
        }
        if self.invincibility != 0 || self.star_high != 0 {
            return (true, None);
        }
        (true, Some(15))
    }

    /// `$1BFF`: the frame's environment probe. The head clears `sSwimming`;
    /// the hazard arm (`$1C4E`) `jp`s the tail when it matches, otherwise the
    /// last sample — the `(x+16, y+28)` tile — selects the water arm
    /// (`$1C82`, `& $FC == $4C`) or the exit-bell arm (`$1CBA`, `& $FE ==
    /// $4A`). Returns the hazard byte for the caller to land at the `$319F`
    /// dispatch.
    fn environment_probe(&mut self, layout: &LevelLayout, keys: &PlayerKeys) -> Option<u8> {
        // $1BFF head: `sPrevSwimming = sSwimming; sSwimming = 0`. The water
        // arm below is the only writer of `$FF` (`sPrevSwimming` feeds its
        // vvel nudge, which the port does not model).
        self.swim = false;
        let (hazard_hit, dead) = self.hazard_probe(layout);
        if hazard_hit {
            return dead;
        }
        if self.detect_swim(layout) {
            // $1C82 / $1CAA: water arm clears sMoonPhysics bit 1
            self.swim = true;
            self.moon &= !0x02;
            return None;
        }
        if self.exit_bell_probe(layout) {
            return None;
        }
        self.moon_zone_probe(layout, keys);
        None
    }

    /// `$1CFB`-`$1D71`: the moon-physics zone probe.
    ///
    /// The last sample (`x+16, y+28`) is tested against `$52..=$57`.
    /// When matched:
    /// - `sMoonPhysics` bit 1 is set
    /// - `sFlutterArmed` is cleared
    /// - `sMoonPhysics` bit 2 (the 3-up moon shield) is cleared
    ///
    /// When not matched:
    /// - `sMoonPhysics` bit 1 is cleared
    ///
    /// If bit 1 changed state (transitioning into or out of a moon zone):
    /// - horizontal velocity `hvel` is clamped to `[72, 191]` (`$48..=$BF`)
    /// - if bit 1 transitioned from 1 to 0 (leaving the moon zone):
    ///   - `vvel` is set to 70 (`$46`)
    ///   - `in_air` is set to true
    ///   - if UP (`keys.up`) is held, `no_jump_cut` is set to true (`$1D64`)
    fn moon_zone_probe(&mut self, layout: &LevelLayout, keys: &PlayerKeys) {
        let t = self.tile(layout, self.x.wrapping_add(16), self.y.wrapping_add(28));
        let old_bit1 = self.moon & 0x02;
        self.moon &= !0x02;
        if (0x52..=0x57).contains(&t) {
            self.moon |= 0x02;
            self.flutter_armed = false;
            self.flutter_charge = 0;
            self.moon &= !0x04;
        }
        let new_bit1 = self.moon & 0x02;
        if new_bit1 != old_bit1 {
            if self.hvel >= 0xC0 {
                self.hvel = 191;
            } else if self.hvel < 0x48 {
                self.hvel = 72;
            }
            if new_bit1 == 0 {
                self.vvel = 70;
                self.in_air = true;
                if keys.up {
                    self.no_jump_cut = true;
                }
            }
        }
    }

    /// `$1CBA`-`$1D01`: the level-exit bell. The last sample (`x+16, y+28`)
    /// matches the `$4A/$4B` pair (`tile & $FE == $4A`); `$4A` is the plain
    /// exit, `$4B` the "route already cleared" twin that leaves
    /// `sExitTileFlag = $FF` (the `$3547` level-tally guard reads it back).
    /// The arm clears `sLowTimeFlag`, requests the level-clear music
    /// (`sMusicRequest = 3`), clears the SFX request bytes
    /// (`sSfxRequest0/1/2`; sfx0/sfx1/sfx2 are tracked here), sets `$FF9B = 24` —
    /// the exit phase, which leaves the in-level loop exactly like the death
    /// fade's `$FF9B = 9` — arms the 192-frame fade, and clears
    /// `sSecretRouteFlag` and `sTileAnimConfig`. Returns true when it fired.
    fn exit_bell_probe(&mut self, layout: &LevelLayout) -> bool {
        let t = self.tile(layout, self.x.wrapping_add(16), self.y.wrapping_add(28));
        if t & 0xFE != 0x4A {
            return false;
        }
        self.low_time_flag = 0;
        self.exit_tile_flag = if t == 0x4B { 0xFF } else { 0 };
        self.music_request = 3;
        self.sfx0 = 0;
        self.sfx1 = 0;
        self.sfx2 = 0;
        self.exit_phase = true;
        self.fade_timer = 192; // sFadeTimerLow/High
        self.secret_route = 0;
        self.tile_anim_config = 0;
        true
    }

    // -----------------------------------------------------------------
    // $1993 / $5960 / $5997 — the brick-bump ("bounce block") system
    // -----------------------------------------------------------------

    /// `$1993`: classify the tile the ceiling probe just bonked and start the
    /// bump. Called from `$15F4`, i.e. after `$15CF` snapped y back and set
    /// vvel 24, and before the pipe re-arm reads `sCollisionTile`.
    ///
    /// Classes (measured on the `$01` block at x=352 with
    /// `probe_block_bump.py`): `$7E`/`$02` → type 3; `$00` → type 2 (small) or
    /// 1 (big); `$03` → type 6 (small) or 5 (big); `$01` → the spin-bounce
    /// flag, a type 3 slot, then `$2C77`/`$591F` +1 coin and the type-4 slot
    /// that draws the coin popping out of the block; `$04`-`$37` → no bump
    /// (still a bonk). The big-Mario arms' `sSfxRequest2` (the destruction
    /// sound) is audio-only and tracked as `sfx2`.
    fn block_bonk(&mut self, tile: u8) {
        self.spin_jump_bounce = 0; // $1993's first store
        match tile {
            0x7E | 0x02 => self.block_bump(3),
            0x00 => {
                // $19B0: small Mario bumps; big Mario destroys the brick —
                // sfx2 and the type-1 bump whose `$5CF1` write turns the cell
                // into the `$60` empty metatile.
                if self.powerup == 0 {
                    self.block_bump(2);
                } else {
                    self.sfx2 = 1;
                    self.block_bump(1);
                }
            }
            0x01 => {
                self.spin_jump_bounce = 0xFF;
                self.sfx0 = 2;
                self.block_alloc(3);
                // $19D2's tail is deferred: `ld a,[sObjectBumpSpecial] / and a
                // / ret nz` decides the coin, and the flag is the object
                // side's `$2F72` (a type-`$80` pose inside the bonk window
                // withholds it). The frame loop resolves it through
                // `resolve_bump_coin` once the pass has run.
                self.bump_coin_pending = true;
            }
            0x03 => {
                // $19C1: small → type 6; big → destroy: sfx2, type 5 (its
                // `$5CF1` write is the `$7F` used-tile).
                if self.powerup == 0 {
                    self.block_bump(6);
                } else {
                    self.sfx2 = 1;
                    self.block_bump(5);
                }
            }
            _ => {}
        }
    }

    /// `$19F1`: the shared tail of the bump classes — sfx, `$2C6C` (the slot
    /// alloc) and the spin-bounce flag.
    fn block_bump(&mut self, ty: u8) {
        self.sfx0 = 2;
        self.block_alloc(ty);
        self.spin_jump_bounce = 0xFF;
    }

    /// `$5960`: the first free slot takes the bump's type, timer 1 and the
    /// bonked cell's rounded centre (`$FFB7`/`$FFB9` = the ceiling probe's
    /// (x+16, y+28|18), left there by `$1DFE`).
    ///
    /// The alloc's tail is `$2F72`: it clears `sObjectBumpSpecial` and arms
    /// the below-hit pass with the probe's low bytes. The engine walks the
    /// object slots there; the port splits the pass at its module seam — the
    /// walk is [`ObjectSim::below_bonk`], which the frame loop runs before
    /// the object engine and hands back as the special flag.
    ///
    /// On no free slot (the `cp $00 / ret z` walk wrapping) the bump is
    /// dropped *and the tail never runs*, so nothing is armed.
    fn block_alloc(&mut self, ty: u8) {
        if !self.alloc_bump_slot(ty) {
            return;
        }
        let (px, py) = self.probe_point;
        self.below_bonk = Some((py as u8, px as u8));
    }

    /// `$5960`'s slot store plus `$2F72`'s `sObjectBumpSpecial` clear — the
    /// part every alloc (including the `$19D2` coin slot's) runs.
    /// Returns false when no slot was free.
    fn alloc_bump_slot(&mut self, ty: u8) -> bool {
        let (px, py) = self.probe_point;
        self.alloc_bump_slot_at(ty, px, py)
    }

    /// The alloc at an explicit bonk point: `$FFB7`/`$FFB9` hold the player's
    /// ceiling probe for `$1993`'s classes, but a fireball's `$06` break
    /// (`$341C`) reaches the same `$2C6C` with the *ball's* post-move Y/X, so
    /// the cell it stores is the ball's own probe cell.
    fn alloc_bump_slot_at(&mut self, ty: u8, px: u16, py: u16) -> bool {
        let Some(slot) = self.block_slots.iter_mut().find(|s| s[0] == 0) else {
            return false;
        };
        slot[0] = ty;
        slot[1] = 1;
        slot[2] = (py as u8 & 0xF0).wrapping_add(8);
        slot[3] = (py >> 8) as u8;
        let (lo, carry) = (px as u8).overflowing_add(8);
        slot[4] = lo & 0xF0;
        slot[5] = ((px >> 8) as u8).wrapping_add(u8::from(carry));
        self.object_bump_special = 0;
        true
    }

    /// `$19D2`'s tail once the object side has run `$2F72` for the bump the
    /// frame armed: `sObjectBumpSpecial` non-zero (a type-`$80` pose sat in
    /// the bonk window) withholds the coin *and* its type-4 slot — exactly
    /// the ROM's `and a / ret nz` before `$2C77` — otherwise `$591F` credits
    /// the BCD counter and the coin-pop slot allocates. A no-op when the
    /// frame armed no bump.
    pub fn resolve_bump_coin(&mut self, special: bool) {
        if !self.bump_coin_pending {
            return;
        }
        self.bump_coin_pending = false;
        self.object_bump_special = if special { 0xFF } else { 0 };
        if self.object_bump_special != 0 {
            return;
        }
        self.coin_credit();
        self.sfx0 = 7;
        self.alloc_bump_slot(4);
    }

    /// `$591F`/`$593D` (the `$01` bonk's credit) and `$1D75`'s tail: one coin
    /// — BCD `inc a` + `daa` with the carry into the high byte, the `$0999`
    /// clamp.
    fn coin_credit(&mut self) {
        let a = bcd_inc(self.coins_low);
        self.coins_low = a;
        if a == 0 {
            self.coins_high = self.coins_high.wrapping_add(1); // the `adc 0`
        }
        if self.coins_high >= 0x0A {
            self.coins_low = 0x99;
            self.coins_high = 0x09;
        }
    }

    /// `$5297`/`$527C`: stage the bump's *own* sprite. The engine writes
    /// `$FFC5 = slotX - sScrollX` (types 1/5 also ∓ the timer) and
    /// `$FFC4 = slotY - sScrollY + offset[timer]`, then enqueues it — so the
    /// bump is drawn at the block while Mario's own sprite keeps his pose.
    ///
    /// This port's sprite path derives its base as `world - cam + 96/98`, while
    /// the engine's bump pair is `slot - sScroll` (no +96/98) and its scroll
    /// sits at `cam - 80` / `cam - 72` (`render_view`'s native-rect origin).
    /// Net anchors, so the renderer reproduces the engine's staged pair
    /// exactly: `x = slotX - 16 (+/- the timer)`, `y = slotY + offset - 26`.
    /// Verified against the console's own staging at the `traj15` bump
    /// (`probe_block_bump.py` logs `$FFC4`/`$FFC5`): slot (472, 368), timer 2,
    /// camera (448, 377) → engine 80/71, port base 80/71.
    fn block_draw_slot(&mut self, i: usize, y_off: i8, frame: u8, x_adj: i16) {
        let s = &self.block_slots[i];
        let sy = ((s[3] as u16) << 8) | s[2] as u16;
        let sx = ((s[5] as u16) << 8) | s[4] as u16;
        let x = sx.wrapping_sub(16).wrapping_add(x_adj as u16);
        let y = sy.wrapping_add(y_off as u16).wrapping_sub(26);
        self.block_draw.push((x, y, frame));
    }

    /// Write one cell of the sparse level-map overlay (`layout_writes`). The
    /// engine edits the same WRAM map every probe reads, so the overlay keeps
    /// the extracted `LevelLayout` immutable while the sim — and the renderer
    /// — sees the rewritten tile from here on.
    fn layout_write(&mut self, row: u8, col: u8, tile: u8) {
        let key = (row as u32) * 256 + col as u32;
        if let Some(w) = self.layout_writes.iter_mut().find(|(k, _)| *k == key) {
            w.1 = tile;
        } else {
            self.layout_writes.push((key, tile));
        }
    }

    /// `$5CF1`: write a tile into the *level map* at the bump's cell (the
    /// engine computes the address from `$FFB7`/`$FFB9`, i.e. the slot's
    /// rounded Y/X). This is the collision-visible half of the bump: the
    /// `$FF`/`$F4-$FB`/`$F8-$FB` 2x2 writes go to the BG map (VRAM) and only
    /// change the art, but this one changes what the probes read. Measured on
    /// the `$01` block at col 22: `$01` → `$24` at timer 2 → `$07` at timer 8,
    /// so a second bonk there hits `$07` — a `$04`+ tile, i.e. a plain bonk
    /// with no bump (the capture's second jump).
    fn block_write_cell(&mut self, i: usize, tile: u8) {
        let s = &self.block_slots[i];
        let sy = (s[3] as u32) << 8 | s[2] as u32;
        let sx = (s[5] as u32) << 8 | s[4] as u32;
        // Slot Y/X store the probe point with +8 and `& $F0`. Recover the cell:
        // Y = (cell top) + 8 → mask the low byte back off (a plain `& $F0` on
        // the 16-bit value would keep the $01 high byte but read $D0, i.e. row
        // 12 instead of 28); col inverts the same storage: (X - 8) >> 4.
        let row = ((sy & 0xFFF0).wrapping_sub(16) >> 4) & 0xFF;
        let col = (sx.wrapping_sub(8) >> 4) & 0xFF;
        self.layout_write(row as u8, col as u8, tile);
    }

    /// `$2C82` → `$5997`: tick every live bump slot. The frame loop calls this
    /// *after* the frame selection (`$2D0A`) and before the object engine, so
    /// its `$FFC6` write is what the drawn sprite — and the capture's `frame`
    /// check — sees; the last drawing slot wins. The timer increments before
    /// the type dispatch, so a fresh slot (timer 1) first draws on its third
    /// tick and frees itself on its eighth (types 2/3/6) or 40th ($28, 1/5) or
    /// 50th ($32, 4). Runs on the non-dead path only (`$0640` skips it), which
    /// is why a bump never disturbs the dead frame 107.
    ///
    /// The per-type level-map tile (`$5CF1`), by type: 1 → 96 ($60, the block
    /// is destroyed and passable), 5 → 127 ($7F, same for the `$03` block),
    /// 2 → 24 then 0, 3 → 24 then 7, 6 → 24 then 3, 4 → never (it is the coin
    /// popping out, pure art). Small Mario's `$00`/`$03` bumps therefore leave
    /// the tile bumpable again; big Mario's destroy it.
    pub fn block_update(&mut self) {
        self.block_frame = None;
        self.block_draw.clear();
        for i in 0..self.block_slots.len() {
            let ty = self.block_slots[i][0];
            if ty == 0 {
                continue;
            }
            let t = self.block_slots[i][1].wrapping_add(1);
            self.block_slots[i][1] = t;
            match ty {
                // $5A72/$5A7A write 96/127 at 2, $5A82/$5A90 write the same
                // again and free the slot at $28; $5A07 (both types share it)
                // draws 101 at x-1 and 102 at x+1, so 102 is the surviving
                // $FFC6.
                1 | 5 => {
                    let tile = if ty == 1 { 96 } else { 127 };
                    match t {
                        2 => self.block_write_cell(i, tile),
                        0x28 => {
                            self.block_write_cell(i, tile);
                            self.block_slots[i][0] = 0;
                        }
                        _ => {
                            // $5A07: the item popping out, two sprites either
                            // side of the block (x ∓ timer), so the pair moves
                            // apart as the timer runs.
                            let off = ITEM_POP_OFFSETS_5A4A[t as usize];
                            self.block_draw_slot(i, off, 101, -(t as i16));
                            self.block_draw_slot(i, off, 102, t as i16);
                            self.block_frame = Some(102);
                        }
                    }
                }
                // $5BFC (all three) writes 24 at timer 2; the removals write
                // the "used block" tile: $5BBC (2) → 0, $5B3C (3) → 7,
                // $5BC8 (6) → 3. $5B8A/$5B06 draw 66.
                2 | 3 | 6 => match t {
                    2 => self.block_write_cell(i, 24),
                    8 => {
                        let tile: u8 = if ty == 3 {
                            7
                        } else if ty == 6 {
                            3
                        } else {
                            0
                        };
                        self.block_write_cell(i, tile);
                        self.block_slots[i][0] = 0;
                    }
                    _ => {
                        self.block_draw_slot(i, BOB_OFFSETS_5BB4[t as usize], 66, 0);
                        self.block_frame = Some(66);
                    }
                },
                // $5A9E: the $5AF9 cycle, `(timer & $0C) >> 2`; $5AFD at $32.
                4 => match t {
                    0x32 => self.block_slots[i][0] = 0,
                    _ => {
                        const CYCLE: [u8; 4] = [0x6D, 0x6E, 0x6F, 0x6E];
                        let frame = CYCLE[((t & 0x0C) >> 2) as usize];
                        self.block_draw_slot(i, COIN_OFFSETS_5AE0[(t >> 1) as usize], frame, 0);
                        self.block_frame = Some(frame);
                    }
                },
                _ => {}
            }
        }
    }

    // -----------------------------------------------------------------
    // $32C1 / $331C / $324F — the fireball (projectile) family
    // -----------------------------------------------------------------
    //
    // Four consecutive tail calls of the player update `$0F71`
    // (`call $3451 / call $32C1 / call $331C / call $324F / ret` at `$0FAE`),
    // so a ball created on the press frame also moves and collides on that
    // frame, and an item consumed this frame (`$3451`) satisfies the spawn's
    // tier gate on the same frame. `$2FED` — the item's own queue entry named
    // it as the spawn — is the *object* pass the draw step runs.

    /// `$32C1`: the B-new-press spawn. Gate order is the ROM's — the press
    /// edge, the fire-flower tier, ducking, the goal bell, then the first free
    /// slot (a press with both live is eaten silently: no sfx, no pose).
    ///
    /// The init reads the player's staged pair (`$FFC0`/`$FFC1` Y,
    /// `$FFC2`/`$FFC3` X): the ball starts 28 px below and 16 px right of
    /// Mario's origin, carrying in both bytes. A spin jump replaces the facing
    /// byte with `($FF97 & 2) - 1` — the same rDiv bit the spin pose reads.
    fn fire_spawn(&mut self, b_pressed: bool) {
        if !b_pressed || self.powerup != 3 || self.ducking || self.goal_bell {
            return;
        }
        let Some(i) = self.fireballs.iter().position(|s| s[F_ACTIVE] == 0) else {
            return;
        };
        let dir = if self.spin_jump != 0 {
            (self.phase & 0x02).wrapping_sub(1)
        } else if self.dir_right {
            0x01
        } else {
            0xFF
        };
        let (ylo, ycarry) = (self.y as u8).overflowing_add(28);
        let (xlo, xcarry) = (self.x as u8).overflowing_add(16);
        let slot = &mut self.fireballs[i];
        slot[F_ACTIVE] = 1;
        slot[F_YLO] = ylo;
        slot[F_YHI] = ((self.y >> 8) as u8).wrapping_add(u8::from(ycarry));
        slot[F_XLO] = xlo;
        slot[F_XHI] = ((self.x >> 8) as u8).wrapping_add(u8::from(xcarry));
        slot[F_DIR] = dir;
        slot[F_PHASE] = 0;
        self.throw_timer = 8;
        self.sfx0 = 11;
    }

    /// `$331C` → `$332E`: the per-slot mover, called after the spawn. X steps
    /// ±3 px every frame (the facing byte doubled and tested against `$80`);
    /// Y follows the `$3439` arc — rising while the phase is >= `$0C`,
    /// descending below it — and two `$1EFA` probes at the ball's own
    /// `(Y-16, X-8)` point bracket the two steps.
    ///
    /// Probe 1 (< `$38`) is terminal: `$06` breaks, every other tile (the `$00`
    /// brick, the `$01` item block) just kills the ball. Probe 2 (< `$48`) pops
    /// a *descending* ball back up — phase `$18`, Y -3 — unless it is the
    /// `$06` break; `$48`-and-up tiles (coins, water, hazards, decor) are
    /// transparent. Both kill paths clear only `+0`: the ROM's
    /// `pop hl / xor a / ld [hl],a` never writes the moved Y/X back.
    fn fire_move(&mut self, layout: &LevelLayout) {
        for i in 0..self.fireballs.len() {
            if self.fireballs[i][F_ACTIVE] != 0 {
                self.fire_slot(i, layout);
            }
        }
    }

    fn fire_slot(&mut self, i: usize, layout: &LevelLayout) {
        let s = self.fireballs[i];
        let mut y = (s[F_YHI] as u16) << 8 | s[F_YLO] as u16;
        let mut x = (s[F_XHI] as u16) << 8 | s[F_XLO] as u16;
        let mut phase = s[F_PHASE];
        // `sla a / cp $80`: only $FF (left, -> $FE) takes the negative arm.
        x = x.wrapping_add(if s[F_DIR].wrapping_shl(1) >= 0x80 {
            -3i16
        } else {
            3
        } as u16);
        let t = self.tile(layout, x, y);
        self.collision_tile = t;
        if t < 0x38 {
            self.fire_break_or_die(i, t, x, y);
            return;
        }
        // The arc step. `$3439[phase]` is only ever read with the phase the
        // slot wrote back, which the popup's own decrement caps at $17.
        let b = FIRE_PHASE_STEP[phase.min(0x17) as usize];
        if phase >= 0x0C {
            y = y.wrapping_sub(b as u16);
        } else {
            y = y.wrapping_add(b as u16);
        }
        let t = self.tile(layout, x, y);
        self.collision_tile = t;
        if t < 0x48 && phase < 0x0C {
            if t == 0x06 {
                self.fire_break_or_die(i, t, x, y);
                return;
            }
            phase = 0x18;
            y = y.wrapping_sub(3);
        }
        phase = phase.saturating_sub(1);
        // `$33FB`: write back `+1..+4` then `+6` — the facing byte is never
        // touched again after the spawn.
        let slot = &mut self.fireballs[i];
        slot[F_YLO] = y as u8;
        slot[F_YHI] = (y >> 8) as u8;
        slot[F_XLO] = x as u8;
        slot[F_XHI] = (x >> 8) as u8;
        slot[F_PHASE] = phase;
    }

    /// `$3415`/`$341C`/`$3430`: the mover's terminal arm. `$06` is the only
    /// breakable tile and it takes the *hardcoded* type-1 bump (`$2C6C`, not
    /// `block_bonk`'s classifier — the player can never bump a `$06`), then
    /// the spin-bounce flag; every other tile below `$38` just dies. Both end
    /// with sfx0 2 and the slot's `+0` cleared, and neither writes the moved
    /// Y/X back.
    fn fire_break_or_die(&mut self, i: usize, tile: u8, x: u16, y: u16) {
        if tile == 0x06 {
            self.sfx2 = 1;
            self.sfx0 = 2;
            // `$5960` runs on `$FFB7`/`$FFB9` — the ball's post-move Y/X — and
            // its `$2F72` tail brings the below-hit pass with it.
            if self.alloc_bump_slot_at(1, x, y) {
                self.fire_below_bonk = Some((y as u8, x as u8));
            }
            self.spin_jump_bounce = 0xFF;
        }
        self.fireballs[i][F_ACTIVE] = 0;
        self.sfx0 = 2;
    }

    /// `$324F` → `$3261`: the per-frame pass over both slots, after the move.
    /// The staged screen pair is `(X - camX + 80, Y - camY + 70)`; a pair whose
    /// high nibble is `$C0` is culled — a silent kill, no sfx, and no object
    /// pass.
    ///
    /// The surviving pair is what `$2FED` tests the object slots against. Those
    /// slots live in the object engine, so the pass splits at that seam: this
    /// stages the pair in [`Self::fire_scan`], the frame loop runs
    /// `ObjectSim::fire_hit` before the object engine and then kills the slot
    /// through [`Self::resolve_fire_hit`]. Inert with no live slot.
    ///
    /// `$3261` also stages the ball's own sprite (`$2CF4`, mapping `$B0`/`$B4`
    /// + `($FF97 & 6) >> 1`), which [`Self::fire_draw`] carries to the renderer.
    fn fire_pass(&mut self) {
        self.fire_draw.clear();
        for i in 0..self.fireballs.len() {
            self.fire_scan[i] = None;
            let s = self.fireballs[i];
            if s[F_ACTIVE] == 0 {
                continue;
            }
            let sx = s[F_XLO].wrapping_sub(self.camx as u8).wrapping_add(80);
            let sy = s[F_YLO].wrapping_sub(self.camy as u8).wrapping_add(70);
            if sx & 0xF0 == 0xC0 || sy & 0xF0 == 0xC0 {
                self.fireballs[i][F_ACTIVE] = 0;
                continue;
            }
            self.fire_scan[i] = Some((sy, sx));
            // `$329E`: $B0 is the left-facing set and $B4 the right; the flicker
            // is rDiv bits 1-2, i.e. `phase` bits 1-2. The ball's X/Y are still
            // the 16-bit values, so the renderer's low-byte pair lands on the
            // staged screen pair exactly.
            let base = if s[F_DIR] == 0xFF { 0xB0 } else { 0xB4 };
            let f = base + ((self.phase & 0x06) >> 1);
            let (fx, fy) = (
                (s[F_XHI] as u16) << 8 | s[F_XLO] as u16,
                (s[F_YHI] as u16) << 8 | s[F_YLO] as u16,
            );
            self.fire_draw
                .push((fx.wrapping_sub(16), fy.wrapping_sub(28), f));
        }
    }

    /// `$3261`'s tail: a non-zero `sObjectHitResult` (raised by the object
    /// side's `$3007`) kills the ball. `sfx0 = 2` for the non-defeat arm is the
    /// object pass's write, applied by the caller that ran it.
    pub fn resolve_fire_hit(&mut self, i: usize, hit: bool) {
        if hit {
            self.fireballs[i][F_ACTIVE] = 0;
        }
    }

    /// `$1E36`→`$1E48` ungated floor probe (no `(y-4) & $F` gate) used by the
    /// pipe travel: `x+12` then `x+20` at `y+44`, tile `< $48`.
    fn floor_probe_ungated(&self, layout: &LevelLayout) -> bool {
        if self.tile(layout, self.x.wrapping_add(12), self.y.wrapping_add(44)) < 0x48 {
            return true;
        }
        self.tile(layout, self.x.wrapping_add(20), self.y.wrapping_add(44)) < 0x48
    }

    /// `$10EC`: the pipe travel. While `sPipeTravelDirection >= $10` the frame
    /// loop (`$0FBB`) runs *only* this — no physics, no landing, no objects.
    /// Returns true while travelling (the caller skips the normal update).
    fn pipe_travel(&mut self, layout: &LevelLayout) {
        match self.pipe_dir {
            // $10: down — keep descending while a ceiling or floor is probed.
            0x10 => {
                self.y = self.y.wrapping_add(2);
                if self.ceiling_hit(layout) || self.floor_probe_ungated(layout) {
                    return;
                }
                self.pipe_dir = 0;
            }
            // $20: up — the entry already applied the first -2.
            0x20 => {
                self.y = self.y.wrapping_sub(2);
                if self.ceiling_hit(layout) || self.floor_probe_ungated(layout) {
                    return;
                }
                self.pipe_dir = 0;
            }
            // $30/$40: sideways — walls gate, walk timer ticks (`$14E3`, b=1).
            0x30 => {
                self.x = self.x.wrapping_add(2);
                self.x_sub = 1;
                self.walk_anim_tick(1);
                if self.right_wall(layout) || self.left_wall(layout) {
                    return;
                }
                self.pipe_dir = 0;
            }
            0x40 => {
                self.x = self.x.wrapping_sub(2);
                self.x_sub = 0xFF;
                self.walk_anim_tick(1);
                if self.right_wall(layout) || self.left_wall(layout) {
                    return;
                }
                self.pipe_dir = 0;
            }
            _ => {}
        }
    }

    /// `$14E3`: walk-animation timer tick (`sMarioWalkAnimTimer += b`, wrap $0C).
    pub fn walk_anim_tick(&mut self, b: u8) {
        let t = self.walk_timer.wrapping_add(b);
        self.walk_timer = if t >= 0x0C { 0 } else { t };
    }

    /// `$1189`: pipe entry, run at the end of the normal player update.
    fn pipe_entry(&mut self, keys: &PlayerKeys) {
        let xlo = self.x as u8;
        let in_window = xlo & 0x10 != 0 && (xlo & 0x0F).wrapping_sub(5) < 7;
        match self.pipe_dir {
            // $1189: stand on a $08/$09 mouth, hold DOWN, X in the window.
            1 => {
                if keys.down && in_window {
                    self.pipe_dir = 0x10;
                    self.sfx0 = 5;
                    self.ducking = false;
                    self.hvel = 0x80;
                }
            }
            // $11C0: hold UP on the mouth — enters with the first -2 applied.
            2 => {
                if keys.up && in_window {
                    self.pipe_dir = 0x20;
                    self.y = self.y.wrapping_sub(2);
                    self.sfx0 = 5;
                    self.ducking = false;
                    self.hvel = 0x80;
                }
            }
            // $11FE/$120E: side pipe, right-hand wall (`$0C`-`$0F` tile on the
            // right). RIGHT (`hKeysHeld` bit 4) starts the $30 travel.
            // If RIGHT is not held, falls through to $1244: LEFT starts $40 travel.
            // (autoscroll + boss guards omitted — unused in non-boss levels)
            3 => {
                if self.side_push_dir == 1 || (keys.right && self.on_ground) {
                    self.pipe_dir = 0x30;
                    self.ducking = false;
                    self.hvel = 0x80;
                    self.sfx0 = 5;
                } else if !keys.right
                    && (self.side_push_dir == 0xFF || (keys.left && self.on_ground))
                {
                    self.pipe_dir = 0x40;
                    self.ducking = false;
                    self.hvel = 0x80;
                    self.sfx0 = 5;
                }
            }
            // $1244: side pipe, left-hand wall; LEFT (`hKeysHeld` bit 5) travels $40.
            4 if self.side_push_dir == 0xFF || (keys.left && self.on_ground) => {
                self.pipe_dir = 0x40;
                self.sfx0 = 5;
                self.ducking = false;
                self.hvel = 0x80;
            }
            _ => {}
        }
    }

    /// `$129F` → `$12CE`: landing response for a floor tile.
    fn landing(&mut self, tile: u8, keys: &PlayerKeys) {
        if tile & 0xFE == 0x04 {
            if tile == 0x05 {
                self.side_push_dir = 0xFF;
                if self.phase & 1 == 1 {
                    self.x = self.x.wrapping_sub(1);
                }
            } else {
                self.side_push_dir = 1;
                if self.phase & 1 == 1 {
                    self.x = self.x.wrapping_add(1);
                }
            }
        }
        // $1327: a spinning Mario ($126F) runs the bonk classification
        // ($1993) on the tile he lands on, and every bump class sets
        // sSpinJumpBounce — so the landing *bounces*: in-air flag $FF, vvel
        // 48, or 75 while A is held (the higher bounce; the ROM reads
        // hKeysHeld bit 0 = A). The `$134F` landing (on-ground = $FF, the
        // pipe re-arm, the `$1366` align nudge) is skipped entirely — the
        // bounce arms `ret` before it. A non-class tile ($04-$37 that is not
        // `$00`/`$01`/`$03`, e.g. plain solid ground) leaves the bounce flag
        // clear and falls through to a normal landing.
        if self.spin_jump != 0 {
            self.block_bonk(tile);
            if self.spin_jump_bounce != 0 {
                self.in_air = true;
                self.vvel = if keys.a { 75 } else { 48 };
                self.on_ground = false;
                return;
            }
        }
        self.on_ground = true;
        // $134F: clear/re-arm the pipe trigger from the landing tile.
        self.pipe_dir = 0;
        if tile & 0xFE == 0x08 {
            self.pipe_dir = 1;
        }
        // $1366: nudge Y down until (y-4) & $F matches the align target (0).
        if ((self.y.wrapping_sub(4) & 0x0F) as u8) != self.pipe_align_target {
            self.y = self.y.wrapping_sub(1);
        }
    }

    /// `home.asm:661` (`$04DD`, the level-init state handler): `sMoonPhysics`
    /// ($A287) is the level's `LevelPropertiesSpace` byte (`$1F91`, indexed by
    /// `sCurLevel`), copied WHOLE — `ld a,[hl] / ld [$A287],a`.
    ///
    /// The byte is a **bitfield**, so this copies the raw byte and never folds
    /// it to a bool (the `sAutoScroll` rule): bit 0 = the level-20 hold-A float,
    /// bit 1 = inside a moon zone (runtime only, `$1BFF`), bit 2 = the moon item
    /// held (`$34E2`), bit 3 = the timed float of the two `$08` levels. ROM
    /// values: index 18 → `$08`, 19 → `$01`, 28 → `$08`, everything else `$00`.
    /// Both frame loops seed through here, so play and replay cannot drift.
    pub fn seed_moon_physics(&mut self, byte: u8) {
        self.moon = byte;
    }

    /// `$0FF2`-`$108F` — the moon jump extension, the FIRST call of the player
    /// update (`$0F7E`, before `$1BFF`, so its bit-1 test reads the zone state
    /// the *previous* frame left). Branch structure transliterated from the
    /// bytes at `baserom.gb` `$0FF2`-`$108F`:
    ///
    /// * `$0FF2` — a ringing goal bell returns first.
    /// * `$0FF7` — `moon == 0` (a plain level) is a no-op.
    /// * bit 0 (`$0FFC` → `$106F`) — level 20's hold-A float: airborne + A held
    ///   → `sVVelocityIndex += 4` per frame, capped at `$48`; an A press
    ///   mid-air requests sfx 18.
    /// * bit 1 (`$1000` → `$1055`) — inside a moon zone: an A *press* sets
    ///   vvel = 52 and, one-shot, `sMoonJumpPoseTimer = 8` + sfx 3.
    /// * bit 3 (`$1004` → `$1029`) — the timed float (levels 19/29): armed to 50
    ///   frames on the ground, dropped when A is released, `vvel += 2` per
    ///   remaining frame capped at `$4A`.
    /// * none of 0/1/3 (`$1008`) — the bit-2-only (moon item) arm: the same
    ///   `vvel += 2` / cap `$4A` / sfx-18-on-press shape with no timer.
    ///
    /// Every arm returns while grounded; only bit 1's arm has no ground test.
    fn moon_jump_extension(&mut self, keys: &PlayerKeys, a_pressed: bool) {
        if self.goal_bell {
            return; // $0FF2: `ld a,[sGoalBell] / and a / ret nz`
        }
        if self.moon == 0 {
            return; // $0FF7: a plain level is a no-op
        }
        if self.moon & 0x01 != 0 {
            // $106F: the level-20 arm.
            if self.on_ground || !keys.a {
                return; // `ld a,[sMarioOnGround] / and a / ret nz`, then `bit 0,a / ret z`
            }
            if a_pressed {
                self.sfx0 = 18; // $107F
            }
            if self.vvel >= 0x48 {
                return; // `cp $48 / ret nc`
            }
            self.vvel = self.vvel.wrapping_add(4); // $108A
            return;
        }
        if self.moon & 0x02 != 0 {
            // $1055: the in-zone re-launch (A press edge only).
            if !a_pressed {
                return; // `ldh a,[hKeysPressed] / bit 0,a / ret z`
            }
            self.vvel = 52; // $105C
            if self.moon_jump_pose_timer != 0 {
                return; // `ld a,[sMoonJumpPoseTimer] / and a / ret nz`
            }
            self.moon_jump_pose_timer = 8; // $1066
            self.sfx0 = 3; // $106B
            return;
        }
        if self.moon & 0x08 != 0 {
            // $1029: the timed float of the two `$08` levels.
            if self.on_ground {
                self.moon_float_timer = 50; // $104A
                return;
            }
            if !keys.a {
                self.moon_float_timer = 0; // $1050
                return;
            }
            if self.moon_float_timer == 0 {
                return; // `and a / ret z`
            }
            self.moon_float_timer -= 1; // $103B
            if self.vvel >= 0x4A {
                return; // `cp $4A / ret nc`
            }
            self.vvel = self.vvel.wrapping_add(2); // $1046
            return;
        }
        // $1008: bit 2 alone (the moon item is held) — no timer, and it ends
        // with its own `ret`, so it never falls into the `$1029` arm.
        if self.on_ground || !keys.a {
            return;
        }
        if a_pressed {
            self.sfx0 = 18; // $1018
        }
        if self.vvel >= 0x4A {
            return; // `cp $4A / ret nc`
        }
        self.vvel = self.vvel.wrapping_add(2); // $1025
    }

    /// The frame's VBlank tail (`home.asm` ~115): `sFadeTimer` is decremented
    /// on **every** in-level frame — `sub 1` on the low byte, `sbc 0` on the
    /// high, clamped to zero on a borrow — and no other part of the level
    /// pass touches it. The port used to move the timer only from the death
    /// and exit fades, so a *scene's* own fade (the arena intro's 352 frames,
    /// `$20EB`) never expired: the boss's parts sat in their intro state
    /// (sprite `$1E`, no walk, no bob) for as long as the fight lasted, and
    /// `$B14E`'s "the fade is idle" gate never opened.
    pub fn tick_fade_timer(&mut self) {
        self.fade_timer = self.fade_timer.saturating_sub(1);
    }

    /// One frame of the player update. `a_pressed`/`b_pressed` are the A/B
    /// new-press edges (`hKeysPressed` bits 0/1).
    pub fn step(
        &mut self,
        keys: &PlayerKeys,
        a_pressed: bool,
        b_pressed: bool,
        layout: &LevelLayout,
    ) {
        self.tick_fade_timer();
        // $0F71/$0FBB: travelling replaces the entire player update.
        if self.pipe_dir >= 0x10 {
            self.pipe_travel(layout);
            return;
        }
        self.phase = self.frame_phase();
        // $13FE
        self.x_sub = 0;
        self.x_pushed = false;
        self.pipe_align_target = 0;
        // $13FE tail: armed pipe directions (1-4) are cleared every frame
        // and must be re-earned by this frame's landing/side-push/entry
        // events — this is what drops the arm when Mario walks off a mouth.
        if self.pipe_dir < 0x10 {
            self.pipe_dir = 0;
        }
        // $0F7E: `$0FF2` — the moon jump extension, the FIRST of `$0F71`'s calls
        // (before `$1BFF`). Its bit-1 test therefore reads the zone state the
        // previous frame's probe left, and its `vvel` writes sit ahead of the
        // gravity below, so the bit-0 cap is reached within the same frame.
        self.moon_jump_extension(keys, a_pressed);
        // $1BFF (water + hazard + exit bell). The probes use the pre-update
        // x/y and last frame's ducking flag, exactly like `$0F71`'s call
        // order; the `sMarioDead` byte the hazard arm returns is applied
        // *after* the tail dispatch below so the hit frame moves normally
        // (the object-contact path gets the same timing from
        // `sml2-runtime`'s post-step write).
        let hazard_dead = self.environment_probe(layout, keys);
        // $129F landing
        if self.vvel < 0x19 {
            self.no_jump_cut = false;
            if self.on_ground_obj {
                self.on_ground = true;
            } else if let Some(t) = self.floor_probe(layout) {
                self.collision_tile = t;
                self.landing(t, keys);
            } else {
                // $129F: no floor. A Mario who was on the ground and is
                // outside the walk band — `hvel - 72 >= $78` in byte
                // arithmetic, i.e. running right (>= $C0) or running left
                // (< $48) — is launched off the ledge with vvel 28.
                if self.on_ground && self.hvel.wrapping_sub(72) >= 0x78 {
                    self.vvel = 28;
                }
                self.on_ground = false;
            }
        }
        // $1413 push-out (+ $143D/$147F side-pipe arming on $0C-$0F tiles).
        // The ±1 lands in x AND in `sMarioXSub`, so `$181C` folds it into the
        // frame's x move a second time (see there). Unreachable in level 1:
        // `sMarioXPushed` (`$A218`) is written only here, and no level-0 capture
        // ever sets it (measured with a temporary trace of `$A218`; wp7f
        // handoff): the level has no side-pipe `$0C-$0F` or side-push `$04/$05`
        // tiles, no camera clamp (`$0FBF`) and no horizontal screen transition
        // — the only ways to get Mario's body inside a solid column.
        if let Some(t) = self.wall_tile(layout, self.x.wrapping_add(11)) {
            if t & 0xFC == 0x0C {
                self.pipe_dir = 3;
            }
            self.x = self.x.wrapping_add(1);
            self.x_sub = self.x_sub.wrapping_add(1);
            self.x_pushed = true;
        } else if let Some(t) = self.wall_tile(layout, self.x.wrapping_add(20)) {
            // ($1464 autoscroll-death check omitted — non-autoscroll levels)
            if t & 0xFC == 0x0C {
                self.pipe_dir = 3;
            }
            self.x = self.x.wrapping_sub(1);
            self.x_sub = self.x_sub.wrapping_sub(1);
            self.x_pushed = true;
        }
        // $14A6 ducking
        self.ducking = keys.down && self.on_ground && !self.goal_bell;
        // $126F spin jump toggle. Engaged while DOWN is held (hKeysHeld bit
        // 7), `sCurPowerup` is odd (bit 0 — mushroom/fire, not small or
        // bunny), Mario is airborne and there is no moon physics; cleared
        // otherwise. The spin sfx (14) fires only on the engaging edge.
        if keys.down && self.powerup & 1 != 0 && !self.on_ground && self.moon & 0x0F == 0 {
            if self.spin_jump == 0 {
                self.sfx0 = 14;
            }
            self.spin_jump = 0xFF;
        } else {
            self.spin_jump = 0;
        }
        // $160A returns immediately while the camera-pan timer is nonzero.
        if self.pan_timer == 0 {
            // $160A jump
            if self.on_ground {
                self.in_air = false;
            }
            let wants_jump = if self.powerup == 2 { keys.a } else { a_pressed };
            if wants_jump && (self.on_ground || self.swim) && !self.goal_bell {
                self.jump_flag = true;
                // $164C: the jump sound, then $164E..$1664 swaps it for the
                // moon/jump-hold variants and $1664..$1674 silences it while
                // UP is held (Bunny Mario keeps it).
                self.sfx0 = 1;
                if self.moon & 0x0F != 0 {
                    self.sfx0 = if self.moon & 0x02 != 0 { 3 } else { 18 };
                } else if self.swim {
                    self.sfx0 = 3;
                }
                if self.powerup != 2 && keys.up {
                    self.sfx0 = 0;
                }
                self.in_air = true;
                self.on_ground = false;
                self.flutter_armed = false;
                self.flutter_charge = 0;
                self.vvel = 75;
                if self.powerup != 2 && (0x48..0xC0).contains(&self.hvel) && !keys.up {
                    self.vvel = 48;
                    if self.swim {
                        self.vvel = 72;
                    }
                }
            }
        }
        // $1090 bunny flutter (gated on camera pan timer == 0, powerup == 2, !goal_bell, moon == 0, vvel < 0x48)
        if !self.goal_bell
            && self.pan_timer == 0
            && self.powerup == 2
            && self.moon == 0
            && self.vvel < 0x48
        {
            if a_pressed {
                self.flutter_armed = true;
                self.flutter = 8;
                self.vvel = 24;
            } else if self.flutter_armed && !self.on_ground {
                if keys.a {
                    self.flutter_charge = self.flutter_charge.wrapping_add(1);
                    if self.flutter_charge >= 12 {
                        self.flutter_charge = 0;
                        self.flutter = 8;
                        self.vvel = 24;
                    }
                } else {
                    self.flutter_charge = 0;
                }
            }
        }
        if self.on_ground {
            self.flutter_charge = 0;
        }
        // $14FA gravity + $156B integrator
        if self.moon & 0x02 == 0 && self.flutter != 0 {
            self.flutter -= 1;
        } else if !self.on_ground {
            let saved_y = self.y;
            if self.vvel >= 0x21 && !self.no_jump_cut && !keys.a {
                self.vvel = 33;
            }
            if self.vvel != 0 {
                self.vvel -= 1;
                if self.vvel == 0x30 {
                    if self.moon == 0 {
                        self.vvel = 24;
                    } else {
                        // $154E: moon extra decrement and apex parity snap
                        self.vvel = self.vvel.saturating_sub(1);
                        if self.vvel < 0x19 {
                            self.vvel = if (self.frame_counter & 1) != 0 {
                                24
                            } else {
                                23
                            };
                        }
                    }
                }
            }
            let table: &[u8] = if self.moon != 0 {
                &SPEED_V_WATER
            } else if self.swim {
                &SPEED_V_GOO
            } else {
                &SPEED_V
            };
            let b = table[self.vvel as usize];
            self.y = self.y.wrapping_add(sdelta(b) as u16);
            if self.vvel < 0x19 {
                if self.on_ground {
                    self.vvel = 24;
                }
            } else if let Some(t) = self.ceiling_tile(layout) {
                // $15B6 → $15CF: the ceiling bonk — snap back to the frame's
                // start Y, vvel = $18, then `$15F4` runs `$1993` (the bump
                // classification) before re-arming the pipe trigger from the
                // bonked tile ($0A/$0B = the up-pipe mouth: dir 2).
                self.y = saved_y;
                self.vvel = 24;
                self.collision_tile = t;
                self.block_bonk(t);
                self.pipe_dir = 0;
                if t & 0xFE == 0x0A {
                    self.pipe_dir = 2;
                }
            } else {
                while self.y >> 8 == 0xFF {
                    self.y = self.y.wrapping_add(1);
                }
            }
        }
        // $1701 horizontal acceleration. The ROM's first instruction here is
        // `ld a,[sGoalBell] / and a / jp nz,$17C5`: while the bell is ringing the
        // input is not merely ignored, it is *replaced*. `$17C5` faces Mario
        // right and re-arms his horizontal velocity every frame — `$B8` grounded,
        // `$80` airborne, so he coasts rather than accelerating — and `$16CC`'s
        // friction below then takes 4 off, which is what makes the coast
        // ~1.4 px/frame: the speed that carries him from the bell into the exit
        // door 56 px away. Skipping the block instead — what this port did — left
        // him frozen where the bell was struck and never reaching it.
        if self.goal_bell {
            self.dir_right = true;
            self.hvel = if self.on_ground { 0xB8 } else { 0x80 };
        } else if self.pan_timer == 0 && !self.ducking && !self.x_pushed {
            if keys.right {
                self.dir_right = true;
                self.hvel = self.hvel.wrapping_add(8);
                if !(self.swim || self.on_ground) {
                    if self.hvel & 0xF0 == 0xC0 {
                        self.hvel = 184;
                    } else if self.hvel >= 0xF4 {
                        self.hvel = 0xF3;
                    }
                } else if self.moon == 0 && keys.b {
                    if self.hvel >= 0xF4 {
                        self.hvel = 0xF3;
                    }
                } else if self.hvel >= 0xC0 {
                    self.hvel = 0xBF;
                }
            } else if keys.left {
                self.dir_right = false;
                self.hvel = self.hvel.wrapping_sub(8);
                if !(self.swim || self.on_ground) {
                    if self.hvel & 0xF0 == 0x40 {
                        self.hvel = 72;
                    } else if self.hvel < 0x08 {
                        self.hvel = 0x08;
                    }
                } else if self.moon == 0 && keys.b {
                    if self.hvel < 0x08 {
                        self.hvel = 0x08;
                    }
                } else if self.hvel < 0x48 {
                    self.hvel = 0x48;
                }
            }
        }
        // $16CC friction
        if self.swim || self.moon & 2 != 0 || self.on_ground {
            let hv = self.hvel & 0xFC;
            if hv == 0x80 {
                self.hvel = 0x80;
            } else if hv < 0x80 {
                self.hvel = self.hvel.wrapping_add(4);
            } else {
                self.hvel = self.hvel.wrapping_sub(4);
            }
        }
        // $181C horizontal integration
        let saved_x = self.x;
        let x_sub_saved = self.x_sub;
        let htable: &[u8] = if self.swim || (self.moon & 2 != 0 && self.on_ground) {
            &SPEED_H_WATER
        } else {
            &SPEED_H
        };
        let b = htable[(self.hvel as usize + (self.phase & 7) as usize) & 0xFF];
        // $3404-$3415: XSub = XSub_old + speed, and x gets the WHOLE new XSub
        // byte — `add 16` / `sub 16` around the 16-bit add is the engine's
        // signed-byte add (the same trick `sdelta` models), and the value added
        // is the accumulated XSub, not the raw table byte. The probe side
        // selector reads XSub's own sign (`bit 7`), so both use it.
        //
        // In a frame with no push-out, XSub_old is 0 ($13FE clears it at the
        // frame start; only `$1413` adds to it before this point), so the add
        // is the speed byte — every capture in the suite takes that path.
        // With a push-out it is ±1 bigger, which is what makes the pushed-out
        // frame move 2 px: `$1413`'s own `x += ±1` plus XSub_new = speed ± 1.
        self.x_sub = self.x_sub.wrapping_add(b);
        self.x = self.x.wrapping_add(sdelta(self.x_sub) as u16);
        if self.x_sub < 0x80 {
            // `$188B`: a wall on the right restores x/XSub and zeroes H speed —
            // and, on the tile that blocked, arms the side pipe when it is a
            // `$0C`-`$0F` mouth. This is the *walk-in* arm ($1413's push-out is
            // the second one, and only runs once Mario is already inside the
            // tile), so without it Mario bounces off level $0D's sideways pipe
            // forever instead of entering it.
            if let Some(t) = self.wall_tile(layout, self.x.wrapping_add(20)) {
                self.x = saved_x;
                self.x_sub = x_sub_saved;
                self.hvel = 0x80;
                if t & 0xFC == 0x0C {
                    self.pipe_dir = 3;
                }
            }
        } else if let Some(t) = self.wall_tile(layout, self.x.wrapping_add(11)) {
            self.x = saved_x;
            self.x_sub = x_sub_saved;
            self.hvel = 0x80;
            if t & 0xFC == 0x0C {
                self.pipe_dir = 3;
            }
        }
        // $0FBF screen-edge wrap
        let sx = self.x.wrapping_sub(self.camx).wrapping_add(96) & 0xFF;
        if sx < 0x10 {
            self.x = self.x.wrapping_add(16 - sx);
        } else if sx >= 0xA0 {
            self.x = self.x.wrapping_sub(sx - 160);
        }
        // $14BF/$14D5 walk-anim timer (+1, +2 with B; wraps at $0C; moon physics and goal bell fix cadence to +1)
        let b = if self.moon != 0 || self.goal_bell {
            1
        } else if keys.b {
            2
        } else {
            1
        };
        let t = self.walk_timer + b;
        self.walk_timer = if t >= 0x0C { 0 } else { t };
        // $1189 pipe entry, then $319F death dispatch (the update's tail).
        self.pipe_entry(keys);
        // The hazard byte lands BEFORE the dispatch — a hazard hit kills in the
        // SAME frame. Measured on the spike death (death3): the console is alive
        // at f5685 and already `dead = $FF` with vvel `$5F` at f5686, because
        // `$1BFF` runs *inside* `$0F71` and `$319F` runs later in the same frame.
        // Nothing between `$1BFF` and `$319F` reads `sMarioDead` (only `$2D7D`,
        // which runs after both), so applying it here is equivalent to applying
        // it at the probe. An *object* contact is different: `$2D7D` writes after
        // `$319F`, so it dispatches one frame later (death1/death2 verify that).
        if let Some(d) = hazard_dead {
            if self.dead == 0 {
                self.dead = d;
                self.sfx0 = if d == 0x80 || self.powerup == 0 { 15 } else { 20 };
            }
        }
        self.death_dispatch();
        // $3451: the item consumer, then the projectile family — the rest of
        // `$0F71`'s tail (`$0FAE`-`$0FB7`). Spawn -> move -> pass is the ROM's
        // order, and the pass only stages: the object scan it feeds runs in the
        // frame loop, before the object engine.
        self.item_consumer();
        self.fire_spawn(b_pressed);
        self.fire_move(layout);
        self.fire_pass();
    }

    /// `$319F`: turn `sMarioDead` into the real death (or a powerup loss).
    /// Runs at the tail of the normal player update, so the frame that gets
    /// hit still moves normally and only the next frame enters the arc.
    fn death_dispatch(&mut self) {
        if self.dead == 0 {
            return;
        }
        if self.dead == 0x80 || self.powerup == 0 {
            // $31D1: small Mario (or the instant-death flag) → the arc.
            self.dead = 0xFF;
            self.walk_timer = 0;
            self.vvel = 95;
            return;
        }
        // $319F: lose a powerup tier + 128 frames of invincibility.
        self.dead = 0;
        self.powerup = if self.powerup == 1 { 0 } else { 1 };
        self.flutter = 0;
        self.flutter_armed = false;
        self.flutter_charge = 0;
        self.invincibility = 128;
    }

    /// `$3451`: consume `sPendingItemId`. The call sits at `$0F71`'s tail
    /// (`call $319F / call $3451 / call $32C1 / …`), so the id staged by the
    /// *previous* frame's object pass takes effect here.
    ///
    /// Id 0 returns **without** clearing the byte (the engine's `and a / ret z`);
    /// every other id ends at `$3539`, which zeroes it. 1/2/3 grant a powerup
    /// (tiers 1/3/2) and fall into `$3534` (`sInvincibilityTimer = 64`); 4 is
    /// the star (timers 4/64, `sMusicRequest = 4`, and the conditional
    /// `sStompChainCount` reset); 5 a 1-UP; 6 the goal bell; 7 the secret
    /// route; 8 the moon; 9 fifty coins to `sCoinLow`. The sfx/music requests
    /// each id makes are audio-only and unmodelled beyond `sSfxRequest0`/
    /// `sSfxRequest1`, which the port tracks (and no capture compares).
    fn item_consumer(&mut self) {
        match self.pending_item {
            0 => return,
            1 => {
                // $3456: mushroom — tier 1, sfx1 20, then `$3534`.
                self.powerup = 1;
                self.sfx1 = 20;
                self.invincibility = 64; // $3534
            }
            2 => {
                // $3467: fire flower — tier 3.
                self.powerup = 3;
                self.sfx1 = 20;
                self.invincibility = 64;
            }
            3 => {
                // $3478: carrot — the bunny tier 2.
                self.powerup = 2;
                self.sfx1 = 20;
                self.invincibility = 64;
            }
            4 => {
                // $3489: star. The `sStompChainCount` clear is conditional:
                // `(sStarTimerLow ^ $FF) | sStarTimerHigh == 0` — only the
                // exhausted (high 0, low $FF) state the `$5D39` decrement
                // stops at resets the chain; a timer that never ran (0, 0)
                // or one still counting does not.
                if self.star_high == 0 && self.star_low == 0xFF {
                    self.stomp_chain_reset = true;
                }
                self.star_high = 4;
                self.star_low = 64;
                self.music_request = 4;
            }
            5 => {
                // $34AF: 1-UP heart — sfx0 9, BCD `inc a / daa` on sLives,
                // the 99 cap when the increment wraps.
                self.sfx0 = 9;
                let a = bcd_inc(self.lives);
                self.lives = if a == 0 { 0x99 } else { a };
            }
            6 => {
                // $34EC: goal bell — sfx0 12, sGoalBell 12, the low-time
                // latch cleared, and vvel lifted to 24 only when below $18.
                self.sfx0 = 12;
                self.goal_bell = true;
                self.low_time_flag = 0;
                if self.vvel < 0x18 {
                    self.vvel = 24;
                }
            }
            7 => {
                // $34C9: secret route.
                self.sfx0 = 21;
                self.secret_route = 0xFF;
            }
            8 => {
                // $34D9: moon physics — sfx1 20 and `set 2`.
                self.sfx1 = 20;
                self.moon |= 0x04;
            }
            9 => {
                // $350A: fifty coins — BCD `add $50 / daa` into sCoinLow,
                // the carry into sCoinHigh, clamped at 999.
                self.sfx0 = 7;
                let (low, carry) = bcd_add_50(self.coins_low);
                self.coins_low = low;
                self.coins_high = self.coins_high.wrapping_add(carry as u8);
                if self.coins_high >= 0x0A {
                    self.coins_low = 0x99;
                    self.coins_high = 0x09;
                }
            }
            _ => {}
        }
        self.pending_item = 0;
    }

    /// Consume the star pickup's one-shot `sStompChainCount` reset if this
    /// frame's `$3451` raised it (see `stomp_chain_reset`).
    pub fn take_stomp_chain_reset(&mut self) -> bool {
        let r = self.stomp_chain_reset;
        self.stomp_chain_reset = false;
        r
    }

    /// `$315C` + `$5550`: one frame of the death arc. The frame loop's dead
    /// branch runs *only* this plus the object engine — the camera is frozen,
    /// and the Y add is 8-bit with NO carry into `$FFC1` (the low byte wraps;
    /// verified live). Ends in the fade (`$3186`: state 9, fade timer 192)
    /// once the draw's screen Y `(y - camY + 98) & $F0 == $B0`.
    pub fn death_arc(&mut self, cam_y: u16) {
        let b = SPEED_V_DEAD[self.vvel as usize];
        self.y = (self.y & 0xFF00) | (self.y as u8).wrapping_add(b) as u16;
        let sy = (self.y as u8).wrapping_sub(cam_y as u8).wrapping_add(98);
        if sy & 0xF0 == 0xB0 {
            self.dead_fade = true;
            self.fade_timer = 192; // $3186
            self.powerup = 0;
            self.invincibility = 0;
            return;
        }
        if self.vvel == 1 {
            return;
        }
        self.vvel -= 1;
    }

    /// One frame of the death fade (`$FF9B = 9`, 192 frames). The call that
    /// reaches 0 is the level-end tail (`$311A`): `$FF9B` moves to `$14`, then
    /// `sLives` is decremented (`sub 1 / daa`) — a borrow means the game runs
    /// state `$05` instead ([`PlayerSim::game_over`]), otherwise the game
    /// leaves for the overworld map.
    ///
    /// The borrow is the `sub 1`'s carry into `daa`, i.e. **`sLives == 0`
    /// exactly**: `daa`'s low-nibble correction (`0x06`, taken whenever H is
    /// set) never touches C, so a ten like `$10` decrements to `$09` without
    /// ending the game. `$311A` writes `$FF9B = $14` *before* the branch, so on
    /// a borrow the `$05` write is what survives — the port keeps only that.
    pub fn fade_step(&mut self) {
        if self.fade_timer == 0 {
            return;
        }
        self.fade_timer -= 1;
        if self.fade_timer == 0 {
            if self.lives == 0 {
                // `sub 1 / daa` on BCD 00 → $99, carry set → `$314B`.
                self.lives = 0x99;
                self.game_over = true;
            } else if self.lives & 0x0F == 0 {
                self.lives = ((self.lives >> 4) - 1) << 4 | 0x09;
            } else {
                self.lives -= 1;
            }
        }
    }

    /// `sGoldenCoin1-6` (`$A84D`, `$A851`, `$A856`, `$A859`, `$A85F`, `$A865`) as
    /// offsets into [`PlayerSim::level_progress`] (`$A848`), which the save record
    /// carries as its `+$0D/+$11/+$16/+$19/+$1F/+$25`. **In `$27AF`'s fold order**:
    /// coins 1, 2, 6, 5, 4, 3 — so element 0 lands in the flags byte's bit 5 and
    /// element 5 in bit 0. That order is the order the castle screen draws them
    /// in, left to right.
    const GOLDEN_COINS: [usize; 6] = [5, 9, 29, 23, 17, 14];

    /// `$27AF`'s fold: `and $80 / sla a / rl b` per coin, in
    /// [`Self::GOLDEN_COINS`] order, giving `sGoldenCoinFlags`.
    ///
    /// The six bytes are the *persisted* ones (the `sLevelProgress` block at
    /// `$A848`), and no in-level code ever sets their bit 7 — the ROM has no
    /// writer for them at all. So in a game played from the start this folds to 0
    /// every time; the mechanism matters because a *loaded save* can carry the
    /// bits, and every consumer tests nothing but bit 7.
    pub fn golden_coin_flags(&self) -> u8 {
        let mut b = 0u8;
        for off in Self::GOLDEN_COINS {
            let bit = (self.level_progress.get(off).copied().unwrap_or(0) & 0x80) >> 7;
            b = (b << 1) | bit;
        }
        b
    }

    /// The collected golden-coin count. `$307BC` (the castle screen) resets
    /// `sGoldenCoinCount` and increments it once per coin with bit 7 set, and
    /// `$27AF` recomputes the same number as the popcount of the flags byte
    /// (`srl a / jr nc / inc c`, six times). Both give this.
    pub fn golden_coin_count(&self) -> u8 {
        self.golden_coin_flags().count_ones() as u8
    }

    /// `$27AF` (state `$05`) — the GAME OVER screen's entry, run by the caller
    /// on the frame after the fade raised [`PlayerSim::game_over`]. Not
    /// modelled: the bank-12 body (the `$4C92` graphics → `$8000`, the `$6492`
    /// tilemap → `$9800`, `sBGPalette = $E4`, `sSavedLCDC = $C3`, and the
    /// `$2874` "GAME OVER" string that `$30884` draws from state `$07`). What
    /// is: `sLives` back to 5, the coin/money counters cleared, the 255-frame
    /// fade armed, and `sMusicRequest = 7` before the hand-off to `$07`.
    ///
    /// Its tail is more than a reset, and is modelled here: it folds the six
    /// golden-coin bytes into `sGoldenCoinFlags`, subtracts that popcount from
    /// the BCD `sLevelTally` (`sub c / daa`), **clears all six bytes**, and calls
    /// `$2934` — the same save the level-clear hand-off makes. So a game over
    /// wipes golden-coin progress and charges the tally for every coin collected.
    pub fn game_over_enter(&mut self) {
        self.lives = 5;
        self.coins_low = 0;
        self.coins_high = 0;
        // The fold has to happen before the clear: `$30884` reads the flags byte
        // on every `$07` frame to stage the first coin's icon.
        self.golden_coin_flags = self.golden_coin_flags();
        self.level_tally = bcd_sub(self.level_tally, self.golden_coin_count());
        for off in Self::GOLDEN_COINS {
            if let Some(b) = self.level_progress.get_mut(off) {
                *b = 0;
            }
        }
        self.fade_timer = 255; // sFadeTimerLow (the high byte is already 0)
        self.music_request = 7;
    }

    /// `$287E` (state `$07`) — one frame of the GAME OVER screen's poll: the
    /// `$27AF` 255-frame fade ticks down (VBlank keeps decrementing
    /// `sFadeTimer`), and once it reads 0 the screen waits for START.
    /// Returns true on the frame that hands back to state `$00` (`Init`), which
    /// also requests `sMusicRequest = $FF`.
    ///
    /// `keys_pressed` is the whole `hKeysPressed` byte, not a flag, because the
    /// ROM compares it *exactly*: `ldh a,[hKeysPressed] / cp $08 / ret nz` — so
    /// holding A or SELECT as well as START does **not** advance. A `bool` here
    /// would quietly accept that input.
    pub fn game_over_step(&mut self, keys_pressed: u8) -> bool {
        if !self.game_over {
            return false;
        }
        if self.fade_timer > 0 {
            self.fade_timer -= 1;
            return false;
        }
        if keys_pressed != 0x08 {
            return false;
        }
        self.game_over = false;
        self.music_request = 0xFF;
        true
    }

    /// `$273E` — the START check at the tail of the in-level frame: START
    /// enters the pause state (`$FF9B = 8`). The ROM stores `sPauseState = 0`
    /// and requests the pause jingle through `sSoundMode = 1`; the port has no
    /// sound-mode state, so that write is flagged rather than invented.
    ///
    /// The ROM's own gate is `hKeysPressed == $08` — START and *nothing else*
    /// newly pressed that frame — plus `sScreenTransition == 0`, and the
    /// in-level loop blanks both key bytes while `sFadeTimer != 0`, so START
    /// is ignored during fades and transitions. The caller supplies the START
    /// edge and the transition/fade state; the "nothing else newly pressed"
    /// half is not modelled, because ENTER is not a gameplay key (flagged).
    pub fn pause_enter(&mut self) {
        self.paused = true;
    }

    /// The palette byte a paused frame shows (`$2756`): the handler tests bit 4
    /// of the `$FF97` counter every frame and writes `$E7` when it is clear
    /// and `$93` when it is set — to *both* `sBGPalette` and `sOAMPalette1`,
    /// so BG and OBJ0 flash together and `sOAMPalette2` (OBP1) is untouched.
    /// There is no timer and no reload value: 16 frames of each value.
    pub fn pause_flash(&self) -> u8 {
        if self.frame_counter & 0x10 == 0 {
            0xE7
        } else {
            0x93
        }
    }

    /// The resume half of `$2756`: any frame with the START press bit set
    /// leaves the pause state and restores `sBGPalette`/`sOAMPalette1`/
    /// `sOAMPalette2` from the `sLevel*` copies (`$A80F-$A811`). The port's
    /// renderer already reads the level palettes from the manifest, so
    /// clearing the flag is the whole restore; `sSoundMode = 2` is flagged.
    pub fn pause_exit(&mut self) {
        self.paused = false;
    }

    /// `$2756`'s SELECT branch: with `sLevelProgressFlags` bit 7 set (the stage
    /// was already cleared) SELECT abandons the level through the `$14`
    /// hand-off instead of resuming — the same entry the level-clear tail
    /// takes. The ROM also writes `sSoundMode = 2`, `sMarioDead = 2` and
    /// clears `sAutoScroll`/`sTileAnimConfig`/`sExitTileFlag`/`sFastMusic`;
    /// the port has no sound-mode or fast-music state and its `dead` field
    /// carries different values ($0F/$80/$FF), so those are flagged. The flag
    /// it does set is [`PlayerSim::exit_completed`], which is this port's
    /// marker that `$FF9B` reached `$14`. Returns whether the branch fired.
    pub fn pause_select_exit(&mut self) -> bool {
        if self.level_progress_flags & 0x80 == 0 {
            return false;
        }
        self.paused = false;
        self.tile_anim_config = 0;
        self.exit_tile_flag = 0;
        self.exit_completed = true;
        true
    }

    /// One frame of the exit fade (`$FF9B = 24`, 192 frames): VBlank's
    /// `sFadeTimer` decrement, which keeps ticking while state `$18`'s
    /// `$3547` poll loop runs. The call that reaches 0 is the `$3547` tail:
    /// `sLevelTally` is incremented in BCD (`add 1 / daa`) unless
    /// `sLevelProgressFlags` bit 7 (stage already cleared) or `sExitTileFlag`
    /// (the `$4B` route) guards it, and `$FF9B` moves to 20 (`$14`,
    /// [`PlayerSim::exit_completed`]). Further calls are no-ops once the
    /// hand-off is marked — the banked `$14` body is not modelled and the
    /// port stays frozen on `exit_phase`.
    pub fn exit_fade_step(&mut self) {
        if !self.exit_phase || self.fade_timer == 0 {
            return;
        }
        self.fade_timer -= 1;
        if self.fade_timer == 0 {
            if self.level_progress_flags & 0x80 == 0 && self.exit_tile_flag == 0 {
                self.level_tally = bcd_inc(self.level_tally);
            }
            self.exit_completed = true;
        }
    }

    /// The `$FF9B = $14` level-clear hand-off. `$2C23` maps bank 15 in and jumps
    /// to its `$6F2B` (file offset `$3EF2B`) body, which the ROM runs **twice**
    /// on a bell clear — hence [`PlayerSim::map_handoff_done`] here and the
    /// `cutscene`/`saves` fields of the outcome.
    ///
    /// * **Pass 1**, only when `sGoalBell` is set (item id 6, `$34EC`): the body
    ///   jumps to `$3F4B1` (even node) or `$3F5A4` (odd) — the post-clear walk
    ///   cutscene: bank-25 gfx, the castle RAM block, `sMapScreenId` 13/14 and
    ///   the item-slot arrays at `$A7D0`. None of that is modelled, because it
    ///   exists to draw the overworld. Two of its effects are: it **clears
    ///   `sGoalBell`**, which is what makes the re-entry at its end take the
    ///   map-return half rather than looping, and the state moves to `$0C`. The
    ///   port has no cutscene, so it collapses those few hundred frames into
    ///   this call.
    /// * **Pass 2**, `$3EF56` → `$3EFB2` → `$3F003` → `$3F03D`: the
    ///   secret-exit node rewrite (the `$4B` tile, live Mario only —
    ///   `$48→$47`, `$05`/`$12`/`$1A→$69`, `$29→$2C`, `$2A→$2F`, `$3C→$40`);
    ///   then `$3EEF0`, which marks the node's `sLevelProgress` entry — bit 7 +
    ///   bit 3 clear on a plain clear, bit 3 set for a dead Mario on the secret
    ///   route, bit 3 cleared for the `$4B` exit — and saves; then the node tail,
    ///   whose `or a / jr nz` keys on the node being **0** (a plain level): it
    ///   sets `sLevelProgress[0]` bit 7 and moves the node to `$14`. The
    ///   `$05`/`$1A` nodes move to the castle node `$3F` instead, and nodes of
    ///   `$1E` and up take map screens this port does not model. Finally `$3F03D`
    ///   saves again and sets `$FF9B = $0C`.
    ///
    /// The overworld this hands off *to* is not ported, so the port keeps
    /// freezing on `exit_phase` afterwards. Its own unmodellable work: the
    /// palette swap (`sBGPalette`/`sOAMPalette1`/`sOAMPalette2` =
    /// `$E4`/`$D0`/`$39` — the map's colours; this port renders the level from
    /// the extracted manifest), the hub/GFX setup at `$3F156`, and the SRAM
    /// writes themselves ([`PlayerSim::save_record`] builds what they would
    /// contain).
    pub fn map_handoff(&mut self, nodes: &crate::map::MapNodes) -> MapHandoffOutcome {
        let mut out = MapHandoffOutcome {
            node: self.map_node,
            ..Default::default()
        };
        if self.map_handoff_done {
            return out;
        }
        self.map_handoff_done = true;

        // Pass 1 — the bell path. `$3EF2B` tests `sGoalBell` FIRST and RETURNS:
        // `call $3F4B1` (or `$3F5A4` when `sMapNode` bit 0 is set — the castle
        // screens), then `ld a,12 / ldh [$FF9B],a / ei / ret`. So none of the
        // node rewrite, `$3EEF0` or the node tail below runs on this path, and a
        // bell-rung level end writes no progress byte and makes no `$2934` save.
        // The port used to fall through here, which the `$1E`-and-up tail being
        // unmodelled masked: node `$48` reported `saves == 0` for the wrong reason.
        if self.goal_bell {
            out.cutscene = true;
            self.goal_bell = false;
            return out;
        }

        // Pass 2 — the secret-exit node rewrite.
        if self.exit_tile_flag != 0 && self.dead == 0 {
            self.map_node = match self.map_node {
                0x48 => 0x47,
                0x05 | 0x12 | 0x1A => 0x69,
                0x29 => 0x2C,
                0x2A => 0x2F,
                0x3C => 0x40,
                node => node,
            };
        }

        // $3EEF0: the node's progress entry, then the save. A node with no
        // progress slot (`$650E >= $FE`) skips both.
        if let Some(slot) = nodes.progress_slot(self.map_node) {
            if let Some(p) = self.level_progress.get_mut(slot as usize) {
                if self.dead != 0 {
                    if self.secret_route == 0xFF {
                        *p |= 0x08;
                        out.saves += 1;
                    }
                } else if self.exit_tile_flag != 0 {
                    *p &= !0x08;
                    out.saves += 1;
                } else {
                    *p |= 0x80;
                    *p &= !0x08;
                    out.saves += 1;
                }
            }
        }

        // $3F003 — the node tail. The ROM splits on `sMapNode >= $1E` FIRST:
        // at or above it every zone/castle node takes the band tree, below it
        // only $05/$1A and node 0 have a branch, and everything else is the
        // plain `$3F03D` tail.
        if self.map_node >= 0x1E {
            self.zone_tail(&mut out);
        } else if self.map_node == 0x05 || self.map_node == 0x1A {
            // $3F014 (`cp $05`/`cp $1A` after the `$1E` test fails): a live Mario
            // moves to the castle node and the cursor jumps to zone screen 4
            // ($3F07E -> `$3D49F` -> sMapScreenId = 4). A dead Mario goes
            // straight to `$3F03D`.
            if self.dead == 0 {
                self.map_node = 0x3F;
                out.screen = Some(4);
                out.music = Some(28);
            } else {
                out.music = Some(6);
            }
        } else if self.map_node == 0 && self.dead == 0 {
            // $3F024: `or a / jr nz,$3F03D` — only node 0 marks the stage cleared
            // and moves to node $14.
            self.level_progress[0] |= 0x80;
            self.map_node = 0x14;
            out.music = Some(6);
        } else {
            out.music = Some(6);
        }
        // `$3F03D` / `$3F147` / `$3F14C`: every tail path above ends in exactly
        // one `call $2934` — the bands through `$3F147` (or `$3F142`'s `$3F14C`),
        // the story nodes through `$3F03D`. The port used to save only when the
        // node was still below `$1E` at this point, which dropped the save for
        // every zone node and for the castle move.
        out.saves += 1;
        out.node = self.map_node;
        out
    }

    /// `$3F04D`-`$3F14C` — the tail for a node at or above `$1E` (every zone and
    /// castle node). Each band selects a zone screen and executes its loader
    /// ($3D0FF/$3D208/$3D371/$3D49F/$3D61F/$3D79A/$3D6EB/$3D880).
    fn zone_tail(&mut self, out: &mut MapHandoffOutcome) {
        use crate::map::progress_offset as po;
        let n = self.map_node;
        match n {
            0x1E..=0x27 => {
                out.screen = Some(2); // $3F04D -> $3D0FF
                out.music = Some(16);
                if self.exit_tile_flag != 0 && self.dead == 0 {
                    self.map_node = 0x25; // 37 ($3D1A4)
                    self.level_progress[po::TREE_A_PATH_1] |= 1;
                }
                if self.map_node == 0x26 {
                    self.map_node = 0x27; // 39 ($3D1B5)
                    self.dead = 0xFF;
                }
            }
            0x28..=0x31 => {
                out.screen = Some(1); // $3F05C -> $3D208
                out.music = Some(30);
                if self.exit_tile_flag != 0 && self.dead == 0 {
                    if self.map_node == 0x2C {
                        self.level_progress[po::PUMPKIN_PATH_1] |= 1; // $3D2B5
                    } else if self.map_node == 0x2F {
                        self.level_progress[po::PUMPKIN_PATH_2] |= 1; // $3D2E8
                    }
                }
            }
            0x32..=0x3B => {
                out.screen = Some(3); // $3F06B -> $3D371
                out.music = Some(5);
                if self.map_node == 0x32 && self.exit_tile_flag != 0 && self.dead == 0 {
                    self.map_node = 0x6B; // 107 ($3D416)
                    self.level_progress[po::MACRO_PATH_0] |= 1;
                }
                if self.map_node == 0x6A && self.dead == 0 {
                    self.map_node = 0x6C; // 108 ($3D446)
                    self.set_gate(po::GATE_MACRO);
                    self.dead = 0xFF;
                }
            }
            0x3C..=0x45 => {
                out.screen = Some(4); // $3F07A -> $3D49F
                out.music = Some(28);
                if self.exit_tile_flag != 0 && self.dead == 0 {
                    self.map_node = 0x43; // 67 ($3D540)
                    self.level_progress[po::SPACE_PATH_0] |= 1;
                }
                if self.map_node == 0x44 {
                    self.map_node = 0x45; // 69 ($3D573)
                    self.dead = 0xFF;
                }
            }
            0x46..=0x4F => {
                out.screen = Some(5); // $3F089 -> $3D61F
                out.music = Some(18);
                if self.exit_tile_flag != 0 && self.dead == 0 {
                    self.level_progress[po::TREE_B_PATH_1] |= 1; // $3D6A7
                }
            }
            0x50..=0x59 => {
                out.screen = Some(6); // $3F098 -> $3D79A
                out.music = Some(27);
            }
            0x5A..=0x63 => {
                if n == 0x5B {
                    self.map_node = 0x5C; // $3F0B1
                }
                out.screen = Some(7); // $3F0B7 -> $3D6EB
                out.music = Some(14);
            }
            // $3F0C2's band: `$66` rewrites to $68 and gates, `$6A`/`$6B`/`$6C`
            // jump back into the screen-3 arm, the rest take `$3F03D`.
            0x66 => {
                self.map_node = 0x68; // $3F0C8
                if self.dead == 0 {
                    self.set_gate(po::GATE_MARIO); // $3F0D4: set 7
                }
                out.music = Some(6);
            }
            0x6A..=0x6C => {
                out.screen = Some(3); // $3F0E4 -> $3F06F -> $3D371
                out.music = Some(5);
                if self.map_node == 0x6A && self.dead == 0 {
                    self.map_node = 0x6C;
                    self.set_gate(po::GATE_MACRO);
                    self.dead = 0xFF;
                }
            }
            0x6E..=0x77 => {
                out.screen = Some(8); // $3F0F6 -> $3D880
                out.music = Some(13);
            }
            // $3F104: `$78` rewrites to $2E and gates, then takes screen 1
            // whether or not Mario is alive.
            0x78 => {
                self.map_node = 0x2E; // $3F10C
                if self.dead == 0 {
                    self.set_gate(po::GATE_PUMPKIN_B); // $3F118
                }
                out.screen = Some(1); // $3F060 -> $3D208
                out.music = Some(30);
            }
            0x7A => {
                self.map_node = 0x31; // $3F12B
                if self.dead == 0 {
                    self.set_gate(po::GATE_PUMPKIN_C); // $3F136
                }
                out.screen = Some(1); // $3F060
                out.music = Some(30);
            }
            // $3F142 (`call $3F156 / jr $3F14C`): a node with no band of its own —
            // no screen, just the tail's save.
            _ => {
                out.music = Some(6);
            }
        }
    }

    /// `set 7, [hl]` on one of the map's gate bytes — an OR into the byte at
    /// `sLevelProgress + off`, which is why it is not a store.
    fn set_gate(&mut self, off: usize) {
        if let Some(b) = self.level_progress.get_mut(off) {
            *b |= 0x80;
        }
    }

    /// Everything the object engine writes back into Mario's state after its
    /// pass, in the ROM's order.
    ///
    /// Both frame loops call this immediately after [`crate::objects::ObjectSim::step`].
    /// They used to carry their own copies of this list, and the play loop's copy
    /// was missing `on_ground_cleared` and `on_ground_obj` outright — a
    /// divergence no capture could show, because only `--play` had it. Keeping it
    /// in one place is what stops that recurring; `keys_a` is `hKeysHeld` bit 0,
    /// because `$2E5C`'s stomp jump is 75 with A held and 48 without.
    pub fn apply_object_result(&mut self, obj: &mut crate::objects::ObjectSim, keys_a: bool) {
        // The bank-2 boss helpers write the sound/music bytes and the
        // boss-clear fade directly (`$9285`'s sfx + `$92C4`'s fade, `$9443`'s
        // music stop); the player sim owns the live bytes, so hand them over
        // the same way the item/stomp fields above are handed over.
        if obj.sfx0 != 0 {
            self.sfx0 = obj.sfx0;
            obj.sfx0 = 0;
        }
        if obj.sfx1 != 0 {
            self.sfx1 = obj.sfx1;
            obj.sfx1 = 0;
        }
        if obj.sfx2 != 0 {
            self.sfx2 = obj.sfx2;
            obj.sfx2 = 0;
        }
        if let Some(track) = obj.music_request.take() {
            self.music_request = track;
        }
        if let Some(t) = obj.fade_timer_set.take() {
            self.fade_timer = t;
        }
        if obj.boss_clear_flag != 0 {
            self.boss_clear = true;
        }
        if obj.boss_clear_handoff && !self.exit_completed {
            obj.boss_clear_handoff = false;
            self.music_request = 15;
            if self.level_progress_flags & 0x80 == 0 {
                self.level_tally = bcd_inc(self.level_tally);
            }
            self.tile_anim_config = 0;
            const BOSS_COIN_OFFSETS: [usize; 25] = [
                0, 0, 0, 0, 0, 5, 0, 0, 0, 9, 0, 0, 0, 29, 0, 0, 23, 0, 0, 17, 0, 0, 0, 14, 0,
            ];
            let level = if obj.level_id != 0 {
                obj.level_id as usize
            } else {
                match self.map_node {
                    0x20 => 5,
                    0x2B => 9,
                    0x53 => 13,
                    0x49 => 16,
                    0x3D => 19,
                    0x35 => 23,
                    _ => 0,
                }
            };
            if level < BOSS_COIN_OFFSETS.len() {
                let off = BOSS_COIN_OFFSETS[level];
                if off != 0 {
                    if let Some(p) = self.level_progress.get_mut(off) {
                        *p |= 0x80;
                    }
                    self.golden_coin_flags = self.golden_coin_flags();
                }
            }
            if self.golden_coin_count() == 6 {
                self.music_request = 26;
            }
            self.exit_phase = true;
            self.exit_completed = true;
        }
        // $2E7C: object interaction can consume moon shield bit 2 and grant invincibility
        if obj.mario.moon_physics != self.moon {
            self.moon = obj.mario.moon_physics;
        }
        if obj.mario.invincibility != self.invincibility {
            self.invincibility = obj.mario.invincibility;
        }
        // The `$8000` dispatcher gate reads `sMarioDead`, and the interaction
        // pass writes it back (15 on a harmful hit).
        if self.dead == 0 && obj.mario_dead != 0 {
            self.dead = obj.mario_dead;
        }
        // `$2DC6`'s item branch stages `sPendingItemId`; the *next* frame's
        // `$3451` (inside `$0F71`) consumes it.
        if obj.pending_item != 0 {
            self.pending_item = obj.pending_item;
            obj.pending_item = 0;
        }
        if let Some(x) = obj.mario_x_adjust.take() {
            self.x = x;
        }
        if let Some(y) = obj.mario_y_adjust.take() {
            self.y = y;
        }
        if obj.on_ground_cleared {
            self.on_ground = false;
        }
        // `$2D7D` clears `sMarioOnObject` at the top of the pass and `$2DF7`
        // raises it to 255 when Mario stands on a bit5 object; a pass the frame
        // loop skips (pipe travel) leaves the byte alone, which is what
        // `objects::step`'s own `!travelling` guard reproduces. Three readers
        // consume it: `$129F`'s landing arm (via `$1378`), the `$5375`/`$53FE`
        // frame test and `$307D`'s pan gate.
        self.on_ground_obj = obj.on_object;
        if obj.stomp_jump {
            self.in_air = true;
            self.vvel = if keys_a { 75 } else { 48 };
        }
        if obj.stomp_bounce {
            self.in_air = true;
            self.vvel = 49;
        }
    }

    /// `$3E1D7`: whether the held direction opens a level at the current node.
    ///
    /// The caller is bank 15's `$3C4D3` — `call $3E1D7 / or a / jp z, $3C5DA` —
    /// so **0 means yes**, and the fall-through is the cursor's walking path.
    /// That polarity is why the hub returns "no": on `sMapScreenId == 0` the
    /// cursor only ever walks, and a level is entered from a zone screen.
    ///
    /// The ROM's four tests, in the order it applies them: `sMapScreenId != 0`;
    /// `$4218[node] != $FE` (a pure path node — note `$FF` is *not* excluded
    /// here); `$6591[slot] & (hKeysHeld & $F0) != 0`, i.e. the held direction
    /// must be one this node offers; and `sLevelProgress[slot]` bit 7 **clear**,
    /// so a stage already cleared is not re-entered through this gate. Entering
    /// then also needs A or START *pressed* (`$3C5F4`'s `hKeysPressed & $09`) —
    /// [`PlayerSim::map_confirm`].
    ///
    /// Two places where the port refuses to imitate the ROM: a node whose
    /// progress slot is `$FE`/`$FF` would make `$6591` read past its table, and
    /// such nodes are `$FE` in `$4218` as well, so the second test normally
    /// returns first — the port returns false rather than modelling the
    /// out-of-bounds read. And with no table loaded everything is false.
    pub fn map_can_enter(
        &self,
        nodes: &crate::map::MapNodes,
        screen_id: u8,
        keys_held: u8,
    ) -> bool {
        crate::map::map_enterable(
            screen_id,
            self.map_node,
            nodes,
            &self.level_progress,
            keys_held,
        )
    }

    /// `$3C5F4`: the A/START confirmation that turns an entry-ready node into
    /// an actual entry — `hKeysPressed & $09` (A is bit 0, START bit 3). The ROM
    /// then dispatches on `sMapNode` to a per-node action, and those handlers are
    /// where the level really loads; they are map-specific (node `$02`/`$19` is
    /// the castle, whose `$3D0D7` is an animation — cursor to 48/158, node 35,
    /// `sMarioDead = 255`, a gfx reload — not a level load) and are not modelled.
    pub fn map_confirm(keys_pressed: u8) -> bool {
        keys_pressed & 0x09 != 0
    }

    /// `$2934`: the 0x50-byte save image. The ROM writes it to
    /// `sSaveSlot0` (`$A000`) + `sSaveFileIndex` * `$50`, so the record itself is
    /// slot-independent; a caller decides where it goes.
    ///
    /// The first 64 bytes are the image at `$A840`, assembled here from what the
    /// port models — `sMapNode` at +0 and `sLevelProgress` at +8. The bytes it
    /// does not model (the cursor at +2..+4, the map's gate bytes, and the six
    /// golden-coin flags at +$0D/+$11/+$16/+$19/+$1F/+$25) stay 0, which for the
    /// golden coins is also what the ROM produces: nothing in it ever sets their
    /// bit 7. Then `sCoinLow`, `sCoinHigh`, `sLevelTally`, `sLives`,
    /// `sMoneyBagCount`, and the running sum of all 69 as the checksum. The five
    /// signature bytes are the fresh-file init's, not `$2934`'s, but a record
    /// without them fails `$28EC`, so they are written here.
    ///
    /// `money_bag` is `sMoneyBagCount` (`$A28D`), which this port keeps on the
    /// object engine (`ObjectSim::money_bag`), so the caller supplies it.
    pub fn save_record(&self, money_bag: u8) -> [u8; 0x50] {
        let mut rec = [0u8; 0x50];
        rec[0] = self.map_node;
        let n = self.level_progress.len().min(0x40 - 0x08);
        rec[0x08..0x08 + n].copy_from_slice(&self.level_progress[..n]);
        rec[0x40] = self.coins_low;
        rec[0x41] = self.coins_high;
        rec[0x42] = self.level_tally;
        rec[0x43] = self.lives;
        rec[0x44] = money_bag;
        rec[0x45] = rec[..0x45].iter().fold(0u8, |c, b| c.wrapping_add(*b));
        rec[0x46..0x4A].copy_from_slice(&crate::map::SAVE_SIGNATURE);
        rec
    }

    /// Restore player and overworld state from a validated 0x50-byte save record.
    pub fn apply_save_record(&mut self, rec: &[u8]) -> bool {
        if !crate::map::save_record_valid(rec) {
            return false;
        }
        self.map_node = rec[0];
        let n = self.level_progress.len().min(0x40 - 0x08);
        self.level_progress[..n].copy_from_slice(&rec[0x08..0x08 + n]);
        self.coins_low = rec[0x40];
        self.coins_high = rec[0x41];
        self.level_tally = rec[0x42];
        self.lives = rec[0x43];
        true
    }

    /// `$0F2A` (level restart) + the fade tail's `$04F1` clears: Mario is put
    /// back at the header spawn with hvel $80 / vvel 24, facing right, with
    /// the death/powerup/invincibility state cleared. Fields the reload does
    /// not touch (on_ground, in_air, moon, the global phase) are left alone.
    pub fn respawn(&mut self, x: u16, y: u16) {
        self.boss_clear = false; // Level-load tail `$05B9`: sBossClearFlag = 0.
        self.x = x;
        self.y = y;
        self.hvel = 0x80;
        self.vvel = 24;
        self.dir_right = true;
        self.walk_timer = 0;
        self.dead = 0;
        self.dead_fade = false;
        self.exit_phase = false;
        self.exit_completed = false;
        self.map_handoff_done = false;
        // `sMapNode` is progress, not level state: the reload leaves it alone.
        self.game_over = false;
        self.paused = false;
        self.pan_timer = 0;
        self.fade_timer = 0;
        self.powerup = 0;
        self.invincibility = 0;
        self.flutter = 0;
        self.flutter_armed = false;
        self.flutter_charge = 0;
        self.goal_bell = false;
        self.pipe_dir = 0;
        self.spin_jump_bounce = 0;
        self.object_bump_special = 0;
        // `$049C` clears the bump slots with the rest of the level scratch, and
        // the level map is re-decompressed, so the bump tile edits go too.
        self.block_slots = [[0; 16]; 8];
        self.block_frame = None;
        self.block_draw.clear();
        self.layout_writes.clear();
        self.below_bonk = None;
        self.bump_coin_pending = false;
        // The boot-time WRAM clear covers `$A880-$A89F` (no per-level writer
        // was found), and a ball must not survive a reload.
        self.fireballs = [[0; 16]; FIRE_SLOTS];
        self.fire_scan = [None; FIRE_SLOTS];
        self.fire_below_bonk = None;
        self.throw_timer = 0;
    }

    fn frame_phase(&self) -> u8 {
        self.phase
    }

    pub fn state(&self) -> PlayerState {
        PlayerState {
            x: self.x,
            y: self.y,
            hvel: self.hvel,
            vvel: self.vvel,
            on_ground: self.on_ground,
            swim: self.swim,
        }
    }

    /// `$5348`: the spin jump's four frame ids, indexed by `($FF97 & $0C) >> 2`.
    /// The ROM keeps them in a 4-byte table immediately after the code that
    /// indexes them.
    const SPIN_JUMP_FRAMES: [u8; 4] = [0x04, 0x10, 0x00, 0x11];

    /// The frame index the draw would put in `$FFC6` (powerup offsets
    /// included). Model: right stand 0 / walk 1-3 / skid 13; left stand 4 /
    /// walk 5-7 / skid 12; duck 14/15; air 8/9; grounded-in-air 10/11;
    /// small +32, bunny +69 (flutter +26 not modelled), fire +112
    /// (throw +32 not modelled). The `$532D` spin pose is modelled from
    /// [`Self::SPIN_JUMP_FRAMES`], but its index needs `$FF97` bit 3, which no
    /// capture records. Moon and pipes (as poses) not modelled.
    pub fn frame_index(&mut self, walk_timer: u8) -> u8 {
        let cell = (walk_timer & 0x0C) >> 2;
        let f = if self.pipe_dir >= 0x10 {
            // $530D: while travelling the base frame comes from the pipe
            // direction — $10/$20 show the spin frame 16, the side states
            // ($30/$40) the walk cycles (`$53AD` +1 / `$5426` +5).
            match self.pipe_dir {
                0x30 => cell + 1,
                0x40 => cell + 5,
                _ => 16,
            }
        } else if self.spin_jump != 0 {
            // $532D: a spin jump overrides every other pose. The ROM tests it
            // *before* `$534C`'s long chain but *after* the pipe-direction
            // branch, so a spinning Mario in a pipe still shows the pipe frame.
            // The index is `($FF97 & 0C) >> 2`: bit 2 comes from `phase`, which
            // the captures record, and bit 3 from `frame_counter`, which they do
            // not — see the note on this in the module header.
            let idx = ((self.phase >> 2) & 1) | (((self.frame_counter >> 3) & 1) << 1);
            Self::SPIN_JUMP_FRAMES[idx as usize]
        } else if self.moon & 0x02 != 0 && !self.on_ground {
            // $534C / $53CF: bit-1 moon zone airborne poses (18/19 right, 20/21 left)
            let timer_active = self.moon_jump_pose_timer != 0;
            if timer_active {
                self.moon_jump_pose_timer -= 1;
            }
            if self.dir_right {
                if timer_active {
                    19
                } else {
                    18
                }
            } else {
                if timer_active {
                    21
                } else {
                    20
                }
            }
        } else if self.ducking {
            if self.dir_right {
                14
            } else {
                15
            }
        } else if self.on_ground || self.on_ground_obj {
            if self.dir_right {
                if self.hvel < 0x80 {
                    // $5384: walking right; the step sound is re-requested every
                    // frame and the slot-0 busy gate turns it into the walk
                    // patter.
                    if self.sfx0 == 0 {
                        self.sfx0 = 4;
                    }
                    13
                } else if self.hvel == 0x80 {
                    0
                } else {
                    1 + cell
                }
            } else if self.hvel >= 0x81 {
                // $53FE: walking left.
                if self.sfx0 == 0 {
                    self.sfx0 = 4;
                }
                12
            } else if self.hvel == 0x80 {
                4
            } else {
                5 + cell
            }
        } else if self.in_air {
            if self.dir_right {
                8
            } else {
                9
            }
        } else if self.dir_right {
            10
        } else {
            11
        };
        // $5447: the invincibility timer decrements once per frame (here, at
        // the engine's frame-selection point); while its bit 2 is set the
        // sprite flashes (+32) and the powerup offset is skipped entirely.
        // If the timer was active but bit 2 is clear, the engine branches to
        // $546B directly, skipping the $545D small-Mario (+32) offset.
        let inv_active = self.invincibility != 0;
        if inv_active {
            self.invincibility -= 1;
            if self.invincibility & 0x04 != 0 {
                return f.wrapping_add(32);
            }
        }
        // `$5447`/`$548A`: the powerup offset, then the fire tier's throw pose.
        // While `sMarioThrowTimer` runs the draw decrements it once and adds a
        // further 32, so a fire Mario shows `+112` normally and `+144` for the
        // eight frames after a spawn (`$32C1` sets 8). Only the tier-3 arm
        // touches the timer, and no capture ever spawns a ball, so the
        // compared `frame` byte is unaffected.
        if self.powerup == 3 && self.throw_timer != 0 {
            self.throw_timer -= 1;
            return f.wrapping_add(112).wrapping_add(32);
        }
        f.wrapping_add(match self.powerup {
            0 if !inv_active => 32,
            2 => 69,
            3 => 112,
            _ => 0,
        })
    }

    /// `$54FB`: while the 3-up moon shield is active (`sMoonPhysics` bit 2),
    /// the draw enqueues an extra satellite/shield frame at Mario's position,
    /// alternating between frames 220 and 221 on a 16-frame cycle (`$FF97 & $10`).
    pub fn moon_shield_frame(&self) -> Option<u8> {
        if self.moon & 0x04 != 0 {
            let alt = (self.frame_counter & 0x10) >> 4;
            Some(220 + alt)
        } else {
            None
        }
    }
}

#[cfg(test)]
#[path = "player/tests/mod.rs"]
mod tests;
