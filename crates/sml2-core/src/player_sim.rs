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
//! Omitted (flagged, not guessed): the `$532D` spin-jump pose (its `$FF97 &
//! $0C` table index needs bit 3, which captures do not record), the goal
//! bell's ring animation, fireball paths, and the `$14` level-clear/map
//! hand-off's banked body (`$3EF2B`: the palette swap, the map-node advance
//! and the save) — the port marks the hand-off reach
//! ([`PlayerSim::exit_completed`]) but keeps freezing. The same applies to the
//! game-over screen's banked body (`$27AF`: the `$4C92` graphics and `$6492`
//! tilemap loads, the `$2874` "GAME OVER" string, the `$2934` save) — the port
//! models its state writes and the `$07` START poll, not the art. Ported
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
    pub camx: u16,
    /// `sCameraY` low byte — `$1BFF`'s hazard arm reads `sMarioScreenY`.
    pub camy: u16,
    /// Mirror of sCollisionTile (last sampled tile).
    pub collision_tile: u8,
    /// The last ceiling probe's sample point — `$1DFE` leaves `$FFB7`/`$FFB9`
    /// at (x+16, y+28|18), and a bump's slot stores that cell's rounded centre.
    pub ceiling_probe: (u16, u16),
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
            exit_completed: false,
            map_node: 0,
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
            ceiling_probe: (0, 0),
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
    fn floor_probe(&self, layout: &LevelLayout) -> Option<u8> {
        if (self.y.wrapping_sub(4) & 0x0F) >= 5 {
            return None;
        }
        let t1 = self.tile(layout, self.x.wrapping_add(12), self.y.wrapping_add(44));
        if t1 < 0x48 {
            return Some(t1);
        }
        let t2 = self.tile(layout, self.x.wrapping_add(20), self.y.wrapping_add(44));
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
        self.ceiling_probe = (px, py);
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
    fn environment_probe(&mut self, layout: &LevelLayout) -> Option<u8> {
        // $1BFF head: `sPrevSwimming = sSwimming; sSwimming = 0`. The water
        // arm below is the only writer of `$FF` (`sPrevSwimming` feeds its
        // vvel nudge, which the port does not model).
        self.swim = false;
        let (hazard_hit, dead) = self.hazard_probe(layout);
        if hazard_hit {
            return dead;
        }
        if self.detect_swim(layout) {
            // $1C9F/$1CAA: the vvel/in_air nudge and the `sMoonPhysics`
            // bit-1 clear are not modelled.
            self.swim = true;
            return None;
        }
        self.exit_bell_probe(layout);
        None
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
        let (px, py) = self.ceiling_probe;
        self.below_bonk = Some((py as u8, px as u8));
    }

    /// `$5960`'s slot store plus `$2F72`'s `sObjectBumpSpecial` clear — the
    /// part every alloc (including the `$19D2` coin slot's) runs.
    /// Returns false when no slot was free.
    fn alloc_bump_slot(&mut self, ty: u8) -> bool {
        let (px, py) = self.ceiling_probe;
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
                self.walk_anim_tick(1);
                if self.right_wall(layout) || self.left_wall(layout) {
                    return;
                }
                self.pipe_dir = 0;
            }
            0x40 => {
                self.x = self.x.wrapping_sub(2);
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
            // $11FE: side pipe, pushed right (`$0C` tile on the left wall).
            // (autoscroll + boss guards omitted — unused in non-boss levels)
            3 => {
                if self.side_push_dir == 1 || (keys.left && self.on_ground) {
                    self.pipe_dir = 0x30;
                    self.ducking = false;
                    self.hvel = 0x80;
                    self.sfx0 = 5;
                }
            }
            // $1244: side pipe, pushed left.
            4 if self.side_push_dir == 0xFF || (keys.right && self.on_ground) => {
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

    /// One frame of the player update. `a_pressed` is the A new-press edge.
    pub fn step(&mut self, keys: &PlayerKeys, a_pressed: bool, layout: &LevelLayout) {
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
        // $1BFF (water + hazard + exit bell). The probes use the pre-update
        // x/y and last frame's ducking flag, exactly like `$0F71`'s call
        // order; the `sMarioDead` byte the hazard arm returns is applied
        // *after* the tail dispatch below so the hit frame moves normally
        // (the object-contact path gets the same timing from
        // `sml2-runtime`'s post-step write).
        let hazard_dead = self.environment_probe(layout);
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
        // $160A jump
        if self.on_ground {
            self.in_air = false;
        }
        let wants_jump = if self.powerup == 2 { keys.a } else { a_pressed };
        if wants_jump && (self.on_ground || self.swim) && !self.goal_bell {
            self.jump_flag = true;
            self.in_air = true;
            self.on_ground = false;
            self.vvel = 75;
            if self.powerup != 2 && (0x48..0xC0).contains(&self.hvel) && !keys.up {
                self.vvel = 48;
                if self.swim {
                    self.vvel = 72;
                }
            }
        }
        // $14FA gravity + $156B integrator
        if !self.on_ground {
            let saved_y = self.y;
            if self.vvel >= 0x21 && !self.no_jump_cut && !keys.a {
                self.vvel = 33;
            }
            if self.vvel != 0 {
                self.vvel -= 1;
                if self.vvel == 0x30 && self.moon == 0 {
                    self.vvel = 24;
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
        // $1701 horizontal acceleration
        if !self.goal_bell && !self.ducking && !self.x_pushed {
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
            if self.right_wall(layout) {
                self.x = saved_x;
                self.x_sub = x_sub_saved;
                self.hvel = 0x80;
            }
        } else if self.left_wall(layout) {
            self.x = saved_x;
            self.x_sub = x_sub_saved;
            self.hvel = 0x80;
        }
        // $0FBF screen-edge wrap
        let sx = self.x.wrapping_sub(self.camx).wrapping_add(96) & 0xFF;
        if sx < 0x10 {
            self.x = self.x.wrapping_add(16 - sx);
        } else if sx >= 0xA0 {
            self.x = self.x.wrapping_sub(sx - 160);
        }
        // $14BF/$14D5 walk-anim timer (+1, +2 with B; wraps at $0C)
        let t = self.walk_timer + if keys.b { 2 } else { 1 };
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
            }
        }
        self.death_dispatch();
        // $3451: the item consumer, last of `$0F71`'s tail calls.
        self.item_consumer();
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

    /// `$27AF` (state `$05`) — the GAME OVER screen's entry, run by the caller
    /// on the frame after the fade raised [`PlayerSim::game_over`]. Not
    /// modelled: the bank-12 body (the `$4C92` graphics → `$8000`, the `$6492`
    /// tilemap → `$9800`, `sBGPalette = $E4`, `sSavedLCDC = $C3`, and the
    /// `$2874` "GAME OVER" string that `$30884` draws from state `$07`). What
    /// is: `sLives` back to 5, the coin/money counters cleared, the 255-frame
    /// fade armed, and `sMusicRequest = 7` before the hand-off to `$07`.
    ///
    /// The tally penalty is `sub c / daa` with c = the set high bits of the six
    /// `sGoldenCoin1-6` bytes ($A84D-$A865) — map-side save bytes with no
    /// in-level writer, so c is always 0 here and the tally is left alone.
    pub fn game_over_enter(&mut self) {
        self.lives = 5;
        self.coins_low = 0;
        self.coins_high = 0;
        self.fade_timer = 255; // sFadeTimerLow (the high byte is already 0)
        self.music_request = 7;
    }

    /// `$287E` (state `$07`) — one frame of the GAME OVER screen's poll: the
    /// `$27AF` 255-frame fade ticks down (VBlank keeps decrementing
    /// `sFadeTimer`), and once it reads 0 the screen waits for START
    /// (`hKeysPressed == $08`). Returns true on the frame that hands back to
    /// state `$00` (`Init`), which also requests `sMusicRequest = $FF`.
    pub fn game_over_step(&mut self, start_pressed: bool) -> bool {
        if !self.game_over {
            return false;
        }
        if self.fade_timer > 0 {
            self.fade_timer -= 1;
            return false;
        }
        if !start_pressed {
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
    /// to its `$6F2B` (file offset `$3EF2B`) body, which the ROM runs for exactly
    /// one frame because it ends by setting `$FF9B = 12` (the overworld map) —
    /// hence [`PlayerSim::map_handoff_done`] here.
    ///
    /// What the body does, and what of it this port can observe:
    ///
    /// * It swaps in the overworld's palettes — `sBGPalette = $E4`,
    ///   `sOAMPalette1 = $D0`, `sOAMPalette2 = $39`. Unobservable here: those
    ///   bytes belong to the map, and this port renders the level from the
    ///   extracted manifest rather than from `sBGPalette`.
    /// * With `sGoalBell` set (item id 6, `$34EC`) it calls `$3F4B1` or
    ///   `$3F5A4` — the castle walk/ending sequences (bank-25 gfx, the castle
    ///   RAM block, `sMapScreenId = 14`) — and then hands to the map. Also the
    ///   map's; only the state move is modelled.
    /// * Otherwise, and only for the `$4B` exit tile on a live Mario, it rewrites
    ///   `sMapNode`, because that exit opens a different node than the level it
    ///   was entered from: `$48 -> $47`, `$05`/`$12`/`$1A -> $69`,
    ///   `$29 -> $2C`, `$2A -> $2F`, `$3C -> $40`.
    /// * The common tail then handles a *plain* level, whose node is 0: it sets
    ///   `sLevelProgressFlags` bit 7 — the "already cleared" bit the tally path
    ///   and the SELECT abandon both read — and moves the node to 20 (`$14`).
    ///   The `$05`/`$1A` nodes instead move to the castle node `$3F`, and nodes
    ///   of `$1E` and up take map-screen branches this port does not model.
    /// * `$2934` is the save. The port has no SRAM, so it is not modelled.
    ///
    /// Returns true the first time it runs.
    pub fn map_handoff(&mut self) -> bool {
        if self.map_handoff_done {
            return false;
        }
        self.map_handoff_done = true;
        if self.goal_bell {
            return true;
        }
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
        match self.map_node {
            // $3F014: a live Mario on the $05/$1A nodes moves to the castle.
            0x05 | 0x1A if self.dead == 0 => self.map_node = 0x3F,
            // $3F024: a plain level — mark the stage cleared, move to node $14.
            0 if self.dead == 0 => {
                self.level_progress_flags |= 0x80;
                self.map_node = 0x14;
            }
            _ => {}
        }
        true
    }

    /// `$0F2A` (level restart) + the fade tail's `$04F1` clears: Mario is put
    /// back at the header spawn with hvel $80 / vvel 24, facing right, with
    /// the death/powerup/invincibility state cleared. Fields the reload does
    /// not touch (on_ground, in_air, moon, the global phase) are left alone.
    pub fn respawn(&mut self, x: u16, y: u16) {
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
        self.fade_timer = 0;
        self.powerup = 0;
        self.invincibility = 0;
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

    /// The frame index the draw would put in `$FFC6` (powerup offsets
    /// included). Model: right stand 0 / walk 1-3 / skid 13; left stand 4 /
    /// walk 5-7 / skid 12; duck 14/15; air 8/9; grounded-in-air 10/11;
    /// small +32, bunny +69 (flutter +26 not modelled), fire +112
    /// (throw +32 not modelled). The `$532D` spin pose (its table index is
    /// `$FF97 & $0C` — bit 3 is not captured), moon and pipes not modelled.
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
        } else if self.ducking {
            if self.dir_right {
                14
            } else {
                15
            }
        } else if self.on_ground || self.on_ground_obj {
            if self.dir_right {
                if self.hvel < 0x80 {
                    13
                } else if self.hvel == 0x80 {
                    0
                } else {
                    1 + cell
                }
            } else if self.hvel >= 0x81 {
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
        if self.invincibility != 0 {
            self.invincibility -= 1;
            if self.invincibility & 0x04 != 0 {
                return f.wrapping_add(32);
            }
        }
        f.wrapping_add(match self.powerup {
            0 => 32,
            2 => 69,
            3 => 112,
            _ => 0,
        })
    }
}

/// Fast unit coverage for the coin touch, the level-map overlay and the BCD
/// credit — the semantics the `traj16` replay verifies against the console,
/// pinned here so a regression fails in `cargo test` without PyBoy.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::{LevelLayout, LAYOUT_SIZE};

    /// Every cell `$60` (the level map's own empty air) — a probe only sees
    /// what a test sets explicitly.
    fn blank_layout() -> LevelLayout {
        LevelLayout::from_bytes(&[0x60; LAYOUT_SIZE]).unwrap()
    }

    fn cell_index(row: usize, col: usize) -> u32 {
        (row * 256 + col) as u32
    }

    /// `$1E8D`'s first (y+32) probe point for a small Mario at (100, 200) with
    /// the probe x at 120: row 13, col 7.
    fn small_sim() -> PlayerSim {
        let mut sim = PlayerSim::new();
        sim.powerup = 0; // small: the y+20 probe is skipped
        sim.x = 100;
        sim.y = 200;
        sim
    }

    /// `$129F`'s second test is `sMarioOnObject`: when it is set the routine
    /// jumps to `$1378`, which forces `sMarioOnGround = 255`, clears
    /// `sPipeTravelDirection` and returns — the floor probe never runs. The
    /// runtime did not set the flag until the object engine's on-object result
    /// was threaded back (before that it was false forever), so this reading of
    /// it was inert: Mario standing on a walker fell through it for the one
    /// frame the probe owned.
    #[test]
    fn on_object_skips_the_floor_probe_and_keeps_mario_grounded() {
        let layout = blank_layout(); // no floor anywhere
        let mut sim = small_sim();
        sim.vvel = 0;
        sim.on_ground = false;
        sim.on_ground_obj = true;
        sim.collision_tile = 0x99; // a sentinel only the probe would overwrite
        sim.step(&PlayerKeys::default(), false, &layout);
        assert!(sim.on_ground, "$1378 sets sMarioOnGround = 255");
        assert_eq!(sim.collision_tile, 0x99, "the probe is skipped entirely");
    }

    /// The control for the test above: with the flag clear the same blank
    /// layout falls through to the probe, finds nothing, and launches the
    /// grounded-but-fast Mario off the ledge (`$129F`: `hvel - 72 >= $78`,
    /// i.e. `hvel >= $C0` — the running-right and running-left bands).
    #[test]
    fn without_on_object_the_blank_layout_launches_him_off_the_ledge() {
        let layout = blank_layout();
        // Running fast ($C0 is the launch band's floor): the launch fires, and
        // the frame's own later +-1 leaves `vvel` at 27, not 28.
        let mut fast = small_sim();
        fast.vvel = 0;
        fast.on_ground = true;
        fast.on_ground_obj = false;
        fast.hvel = 0xC0;
        fast.step(&PlayerKeys::default(), false, &layout);
        assert!(!fast.on_ground, "nothing under him and nothing to stand on");
        assert_eq!(
            fast.vvel, 27,
            "$129F sets 28; the rest of the frame leaves 27 (hvel {})",
            fast.hvel
        );
        // Ambling at the neutral speed, below the band: no launch at all.
        let mut slow = small_sim();
        slow.vvel = 0;
        slow.on_ground = true;
        slow.on_ground_obj = false;
        slow.hvel = 0x80;
        slow.step(&PlayerKeys::default(), false, &layout);
        assert_eq!(slow.vvel, 0, "below $C0 the launch does not fire");
    }

    /// `$5375`/`$53FE`: the walk-cycle arm's test is
    /// `sMarioOnGround | sMarioOnObject`, so a Mario standing on an object
    /// draws the standing/walk pose with `sMarioOnGround` clear — the state the
    /// runtime left him in while it never set the flag, so the air pose won.
    #[test]
    fn on_object_draws_the_walk_pose_with_ground_clear() {
        let mut sim = small_sim();
        sim.in_air = true; // he is standing on something that is not the floor
        sim.on_ground = false;
        sim.hvel = 0x80;
        sim.dir_right = true;
        sim.on_ground_obj = true;
        assert_eq!(sim.frame_index(0), 32, "the standing pose, small (+32)");
        sim.on_ground_obj = false;
        assert_eq!(sim.frame_index(0), 40, "the air pose (+32) wins instead");
    }

    #[test]
    fn coin_touch_credits_clears_the_cell_and_is_not_a_wall() {
        let mut layout = blank_layout();
        layout.cells[13 * 256 + 7] = 0x48;
        let mut sim = small_sim();
        assert_eq!(sim.wall_tile(&layout, 120), None); // $48 is not solid
        assert_eq!(sim.sfx0, 7);
        assert_eq!(sim.coins_low, 0x01);
        assert_eq!(sim.coins_high, 0x00);
        assert_eq!(sim.layout_writes, vec![(cell_index(13, 7), 0x60)]);
        // The overlay is what the next probe reads: no double credit.
        assert_eq!(sim.wall_tile(&layout, 120), None);
        assert_eq!(sim.coins_low, 0x01);
        assert_eq!(sim.layout_writes.len(), 1);
    }

    /// `$1DA5`'s `cp $48` is exact: a `$49` (the `& $FE == $48` door) collects
    /// too, but writes the `$7F` tile, not `$60`.
    #[test]
    fn coin_touch_writes_7f_for_the_49_tile() {
        let mut layout = blank_layout();
        layout.cells[13 * 256 + 7] = 0x49;
        let mut sim = small_sim();
        assert_eq!(sim.wall_tile(&layout, 120), None);
        assert_eq!(sim.coins_low, 0x01);
        assert_eq!(sim.layout_writes, vec![(cell_index(13, 7), 0x7F)]);
    }

    /// Big Mario probes y+20 first — (120, 220) = row 12, col 7 here.
    #[test]
    fn coin_touch_runs_on_the_y20_probe_too() {
        let mut layout = blank_layout();
        layout.cells[12 * 256 + 7] = 0x48;
        let mut sim = PlayerSim::new(); // `$0F2A` default: big, not ducking
        sim.x = 100;
        sim.y = 200;
        assert_eq!(sim.wall_tile(&layout, 120), None);
        assert_eq!(sim.coins_low, 0x01);
        assert_eq!(sim.layout_writes, vec![(cell_index(12, 7), 0x60)]);
    }

    /// A solid tile still ends the probe as a wall and collects nothing.
    #[test]
    fn solid_tile_still_wins_over_the_touch() {
        let mut layout = blank_layout();
        layout.cells[13 * 256 + 7] = 0x10;
        let mut sim = small_sim();
        assert_eq!(sim.wall_tile(&layout, 120), Some(0x10));
        assert_eq!(sim.coins_low, 0x00);
        assert!(sim.layout_writes.is_empty());
    }

    /// The BCD credit: 09 -> 10, 99 + carry -> 00 with the high byte, and the
    /// `$0999` clamp.
    #[test]
    fn coin_credit_is_bcd_and_caps_at_0999() {
        let mut sim = PlayerSim::new();
        sim.coins_low = 0x09;
        sim.coin_credit();
        assert_eq!((sim.coins_high, sim.coins_low), (0x00, 0x10));
        sim.coins_low = 0x99;
        sim.coin_credit();
        assert_eq!((sim.coins_high, sim.coins_low), (0x01, 0x00));
        sim.coins_high = 0x09;
        sim.coins_low = 0x99;
        sim.coin_credit();
        assert_eq!((sim.coins_high, sim.coins_low), (0x09, 0x99));
    }

    // -----------------------------------------------------------------
    // $1BFF's exit-bell arm ($1CBA-$1D01). Level 1's only `$4A` sits at the
    // very east end (col 254 / row 30, world x 4064) — past every capture's
    // range — so the arm is pinned here rather than by a replay.
    // -----------------------------------------------------------------

    /// For a small Mario at (100, 200) the `(x+16, y+28)` environment sample
    /// is (116, 228) = row 13, col 6.
    const EXIT_CELL: usize = 13 * 256 + 6;

    /// `$4A` (the plain exit): the arm clears everything it owns, requests
    /// the level-clear music and starts the 192-frame exit fade.
    #[test]
    fn exit_bell_4a_starts_the_level_clear() {
        let mut layout = blank_layout();
        layout.cells[EXIT_CELL] = 0x4A;
        let mut sim = small_sim();
        // Pre-seed every byte the arm must overwrite.
        sim.sfx0 = 20;
        sim.sfx1 = 20;
        sim.music_request = 9;
        sim.low_time_flag = 0xFF;
        sim.secret_route = 0xFF;
        sim.tile_anim_config = 0x31;
        assert!(sim.exit_bell_probe(&layout));
        assert_eq!(sim.exit_tile_flag, 0x00);
        assert_eq!(sim.low_time_flag, 0);
        assert_eq!(sim.music_request, 3);
        assert_eq!(sim.sfx0, 0);
        assert_eq!(sim.sfx1, 0);
        assert!(sim.exit_phase);
        assert_eq!(sim.fade_timer, 192);
        assert_eq!(sim.secret_route, 0);
        assert_eq!(sim.tile_anim_config, 0);
    }

    /// `$4B` is the "route already cleared" twin: same arm, but
    /// `sExitTileFlag` becomes `$FF` (`$3547`'s tally guard reads it back).
    #[test]
    fn exit_bell_4b_sets_the_already_cleared_flag() {
        let mut layout = blank_layout();
        layout.cells[EXIT_CELL] = 0x4B;
        let mut sim = small_sim();
        assert!(sim.exit_bell_probe(&layout));
        assert_eq!(sim.exit_tile_flag, 0xFF);
        assert!(sim.exit_phase);
        assert_eq!(sim.fade_timer, 192);
    }

    /// The arm is the `$4A/$4B` pair exactly — the neighbouring coin
    /// (`$48/$49`) and water (`$4C-$4F`) classes are not exits, and neither
    /// is anything else.
    #[test]
    fn exit_bell_needs_the_exact_4a_4b_pair() {
        for tile in [0x00, 0x38, 0x48, 0x49, 0x4C, 0x4D, 0x52, 0x60, 0x7E] {
            let mut layout = blank_layout();
            layout.cells[EXIT_CELL] = tile;
            let mut sim = small_sim();
            assert!(
                !sim.exit_bell_probe(&layout),
                "tile ${tile:02X} must not arm the exit"
            );
            assert!(!sim.exit_phase);
        }
    }

    /// `$1BFF`'s tail order: the water arm reads the same sample, so a `$4C`
    /// tile swims and never arms the exit.
    #[test]
    fn water_beats_the_exit_arm_at_the_same_sample() {
        let mut layout = blank_layout();
        layout.cells[EXIT_CELL] = 0x4C;
        let mut sim = small_sim();
        assert_eq!(sim.environment_probe(&layout), None);
        assert!(sim.swim);
        assert!(!sim.exit_phase);
    }

    /// `$1BFF` never collects: a `$48/$49` at the environment sample arms no
    /// exit and credits no coin (only the side probes' `$1D75` collects).
    #[test]
    fn exit_sample_never_credits_a_coin() {
        let mut layout = blank_layout();
        layout.cells[EXIT_CELL] = 0x49;
        let mut sim = small_sim();
        assert_eq!(sim.environment_probe(&layout), None);
        assert_eq!(sim.coins_low, 0);
        assert!(sim.layout_writes.is_empty());
        assert!(!sim.exit_phase);
    }

    /// The hazard arm `jp`s the tail, so a `$50/$51` tile at the big-Mario
    /// `y+22` probe blocks the bell even with a `$4A` one row below.
    #[test]
    fn hazard_arm_blocks_the_exit_probe() {
        let mut layout = blank_layout();
        layout.cells[12 * 256 + 6] = 0x50; // y+22 → the big-Mario hazard probe
        layout.cells[EXIT_CELL] = 0x4A; // y+28 → a bell one row down
        let mut sim = PlayerSim::new(); // $0F2A default: big, not ducking
        sim.x = 100;
        sim.y = 200;
        assert_eq!(sim.environment_probe(&layout), Some(15));
        assert!(!sim.exit_phase);
        assert_eq!(sim.exit_tile_flag, 0);
    }

    /// `$1C39`: big Mario falls through from the `y+22` hazard probe to
    /// `y+28`, so a `$51` one row below still kills (and still blocks the
    /// exit arm). Every capture runs small, so no replay reaches this arm.
    #[test]
    fn hazard_probe_falls_through_to_the_y28_sample() {
        let mut layout = blank_layout();
        layout.cells[EXIT_CELL] = 0x51;
        let mut sim = PlayerSim::new();
        sim.x = 100;
        sim.y = 200;
        assert_eq!(sim.environment_probe(&layout), Some(15));
        assert!(!sim.exit_phase);
    }

    /// The arm through the real per-frame entry point: `step` arms the level
    /// clear without moving Mario.
    #[test]
    fn step_arms_the_exit_bell_in_place() {
        let mut layout = blank_layout();
        layout.cells[EXIT_CELL] = 0x4A;
        let mut sim = small_sim();
        let keys = PlayerKeys::default();
        sim.step(&keys, false, &layout);
        assert!(sim.exit_phase);
        assert_eq!(sim.fade_timer, 192);
        assert_eq!(sim.music_request, 3);
        assert_eq!(sim.exit_tile_flag, 0);
    }

    // -----------------------------------------------------------------
    // $3547's tail ($18): the exit fade's last step runs the BCD level
    // tally and hands off to `$14` ($FF9B = 20). Level 1's only `$4A` is
    // past every capture's east end, so the whole tail is pinned here.
    // -----------------------------------------------------------------

    /// A plain `$4A` exit: nothing is credited while the 192-frame fade
    /// runs; the call that reaches 0 steps `sLevelTally` 0 -> 1 and marks
    /// the `$14` hand-off. Past the tail the step is a no-op.
    #[test]
    fn exit_fade_tally_counts_a_plain_4a_exit() {
        let mut layout = blank_layout();
        layout.cells[EXIT_CELL] = 0x4A;
        let mut sim = small_sim();
        assert!(sim.exit_bell_probe(&layout));
        assert_eq!(sim.fade_timer, 192);
        // The arm frame itself never decrements (VBlank ticked before the
        // in-level update set 192), so the 191 calls leave the timer at 1.
        for _ in 0..191 {
            sim.exit_fade_step();
        }
        assert_eq!(sim.fade_timer, 1);
        assert_eq!(sim.level_tally, 0);
        assert!(!sim.exit_completed);
        // The 192nd call is `$3547`'s tail.
        sim.exit_fade_step();
        assert_eq!(sim.fade_timer, 0);
        assert_eq!(sim.level_tally, 0x01);
        assert!(sim.exit_completed);
        // No second credit once the hand-off is marked.
        sim.exit_fade_step();
        assert_eq!(sim.level_tally, 0x01);
    }

    /// `add 1 / daa`: the tally is BCD, including the 99 -> 00 wrap (the
    /// carry is discarded — `sLevelTally` is a single byte).
    #[test]
    fn exit_fade_tally_is_bcd() {
        for (from, to) in [(0x00u8, 0x01u8), (0x09, 0x10), (0x19, 0x20), (0x99, 0x00)] {
            let mut sim = small_sim();
            sim.exit_phase = true;
            sim.fade_timer = 1;
            sim.level_tally = from;
            sim.exit_fade_step();
            assert_eq!(
                sim.level_tally, to,
                "tally ${from:02X} must step to ${to:02X}"
            );
            assert!(sim.exit_completed);
        }
    }

    /// The `$4B` ("route already cleared") exit sets `sExitTileFlag = $FF`,
    /// and `$3547`'s `and a / ret nz` guard skips the tally — but the `$14`
    /// hand-off still happens.
    #[test]
    fn exit_fade_tally_skips_the_already_cleared_4b_exit() {
        let mut layout = blank_layout();
        layout.cells[EXIT_CELL] = 0x4B;
        let mut sim = small_sim();
        sim.level_tally = 0x07;
        assert!(sim.exit_bell_probe(&layout));
        assert_eq!(sim.exit_tile_flag, 0xFF);
        for _ in 0..192 {
            sim.exit_fade_step();
        }
        assert_eq!(sim.fade_timer, 0);
        assert_eq!(sim.level_tally, 0x07);
        assert!(sim.exit_completed);
    }

    /// `sLevelProgressFlags` bit 7 (the stage already cleared) guards the
    /// tally too — and only bit 7: a flags byte with other bits set and bit
    /// 7 clear still counts.
    #[test]
    fn exit_fade_tally_skips_when_progress_bit7_is_set() {
        let mut layout = blank_layout();
        layout.cells[EXIT_CELL] = 0x4A;
        let mut sim = small_sim();
        sim.level_tally = 0x12;
        sim.level_progress_flags = 0x80;
        assert!(sim.exit_bell_probe(&layout));
        for _ in 0..192 {
            sim.exit_fade_step();
        }
        assert_eq!(sim.level_tally, 0x12);
        assert!(sim.exit_completed);
        // Other flags bits are not the guard: bit 0 alone still counts.
        sim.exit_completed = false;
        sim.level_progress_flags = 0x01;
        sim.exit_phase = true;
        sim.fade_timer = 1;
        sim.exit_fade_step();
        assert_eq!(sim.level_tally, 0x13);
    }

    /// Outside the exit phase the step must not touch the timer (the death
    /// fade runs its own `fade_step`), and a zeroed timer must not credit.
    #[test]
    fn exit_fade_step_needs_the_exit_phase() {
        let mut sim = small_sim();
        sim.fade_timer = 5;
        sim.exit_fade_step();
        assert_eq!(sim.fade_timer, 5);
        assert_eq!(sim.level_tally, 0);
        assert!(!sim.exit_completed);
    }

    // -----------------------------------------------------------------
    // $14 — the level-clear/map hand-off ($2C23 -> bank 15 $6F2B).
    //
    // The gate cannot see any of this: no capture runs past the exit fade
    // (`$FF9B = $14` freezes the port), so every branch is pinned here.
    // -----------------------------------------------------------------

    /// Level 1's own case: the `$4A` exit bell sets neither `sGoalBell` nor
    /// `sExitTileFlag`, and a level entered from the map has node 0 — which is
    /// exactly the branch that marks the stage cleared and moves the node on.
    #[test]
    fn map_handoff_marks_a_plain_level_cleared() {
        let mut sim = small_sim();
        assert_eq!(sim.map_node, 0, "a fresh sim is on node 0");
        assert!(!sim.goal_bell && sim.exit_tile_flag == 0 && sim.dead == 0);

        assert!(sim.map_handoff(), "the body runs once");
        assert_eq!(sim.level_progress_flags & 0x80, 0x80, "$3F024 sets bit 7");
        assert_eq!(sim.map_node, 0x14, "and moves the node to 20");
        assert!(
            !sim.map_handoff(),
            "the ROM sets $FF9B = 12, so it runs once"
        );
        assert_eq!(sim.map_node, 0x14);
    }

    /// The `$4B` secret exit rewrites the node: that exit opens a different
    /// node than the one the level was entered from.
    #[test]
    fn map_handoff_rewrites_the_node_for_the_secret_exit() {
        for (from, want) in [
            (0x48u8, 0x47u8),
            (0x05, 0x69),
            (0x12, 0x69),
            (0x1A, 0x69),
            (0x29, 0x2C),
            (0x2A, 0x2F),
            (0x3C, 0x40),
        ] {
            let mut sim = small_sim();
            sim.map_node = from;
            sim.exit_tile_flag = 0xFF;
            assert!(sim.map_handoff());
            assert_eq!(sim.map_node, want, "node ${from:02X}");
        }
        // A node the chain does not know is left alone, and — being neither 0
        // nor $05/$1A — takes none of the tail's branches either.
        let mut sim = small_sim();
        sim.map_node = 0x07;
        sim.exit_tile_flag = 0xFF;
        sim.map_handoff();
        assert_eq!(sim.map_node, 0x07);
        assert_eq!(sim.level_progress_flags & 0x80, 0);
    }

    /// The bell path (item id 6 set `sGoalBell`) hands to the map through the
    /// castle sequences and never touches the node or the progress bit.
    #[test]
    fn map_handoff_bell_path_skips_the_node_work() {
        let mut sim = small_sim();
        sim.goal_bell = true;
        sim.exit_tile_flag = 0xFF; // would otherwise rewrite the node
        sim.map_node = 0x48;
        assert!(sim.map_handoff());
        assert_eq!(sim.map_node, 0x48);
        assert_eq!(sim.level_progress_flags & 0x80, 0);
    }

    /// Every branch of the body is gated on a live Mario (`sMarioDead`).
    #[test]
    fn map_handoff_needs_a_live_mario() {
        let mut sim = small_sim();
        sim.dead = 0x0F;
        sim.exit_tile_flag = 0xFF;
        sim.map_node = 0x48;
        sim.map_handoff();
        assert_eq!(sim.map_node, 0x48, "the rewrite is skipped");
        assert_eq!(sim.level_progress_flags & 0x80, 0, "and so is the tail");

        // Node 0 with a dead Mario: still nothing.
        let mut dead_plain = small_sim();
        dead_plain.dead = 0x80;
        dead_plain.map_handoff();
        assert_eq!(dead_plain.map_node, 0);
        assert_eq!(dead_plain.level_progress_flags & 0x80, 0);
    }

    /// The `$05`/`$1A` nodes move to the castle node `$3F` — but only when the
    /// secret-exit rewrite has not already sent them to `$69`.
    #[test]
    fn map_handoff_moves_the_castle_nodes() {
        for from in [0x05u8, 0x1A] {
            let mut sim = small_sim();
            sim.map_node = from;
            sim.map_handoff();
            assert_eq!(sim.map_node, 0x3F, "node ${from:02X} -> the castle");
        }
        let mut secret = small_sim();
        secret.map_node = 0x05;
        secret.exit_tile_flag = 0xFF;
        secret.map_handoff();
        assert_eq!(secret.map_node, 0x69, "the $4B rewrite wins");
    }

    /// A level reload clears the port's own marker but leaves `sMapNode` alone:
    /// the node is progress, not level state.
    #[test]
    fn a_level_reload_keeps_the_map_node() {
        let mut sim = small_sim();
        sim.map_handoff();
        assert_eq!(sim.map_node, 0x14);
        sim.respawn(112, 240);
        assert!(!sim.map_handoff_done, "the marker is a phase flag");
        assert_eq!(sim.map_node, 0x14, "the node survives the reload");
    }

    // -----------------------------------------------------------------
    // $3451 / $3534 — the item consumer (powerup transitions).
    //
    // The gate captures only ever reach id 7 (`traj13`/`traj14`'s `secret`
    // check stages a `$87` object; the rest of the table is unexercised), so
    // every transition is pinned here: the powerup grants + `$3534` timers,
    // the BCD adds, the goal-bell tail, and — because the hazard/ceiling/
    // side probes read `sCurPowerup` back — that a mushroom really moves
    // the hitboxes.
    // -----------------------------------------------------------------

    /// Everything `$3451` can write, in one comparable snapshot (sfx0/sfx1,
    /// powerup, the timers, the caps and the flags).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct ConsumerState {
        powerup: u8,
        invincibility: u8,
        lives: u8,
        coins_low: u8,
        coins_high: u8,
        star_high: u8,
        star_low: u8,
        moon: u8,
        secret_route: u8,
        goal_bell: bool,
        low_time_flag: u8,
        music_request: u8,
        sfx0: u8,
        sfx1: u8,
    }

    fn consumer_state(s: &PlayerSim) -> ConsumerState {
        ConsumerState {
            powerup: s.powerup,
            invincibility: s.invincibility,
            lives: s.lives,
            coins_low: s.coins_low,
            coins_high: s.coins_high,
            star_high: s.star_high,
            star_low: s.star_low,
            moon: s.moon,
            secret_route: s.secret_route,
            goal_bell: s.goal_bell,
            low_time_flag: s.low_time_flag,
            music_request: s.music_request,
            sfx0: s.sfx0,
            sfx1: s.sfx1,
        }
    }

    /// The engine's `and a / ret z`: with the byte already 0 nothing runs.
    #[test]
    fn item_id_zero_is_a_no_op() {
        let mut sim = PlayerSim::new();
        sim.sfx0 = 3;
        sim.sfx1 = 4;
        sim.low_time_flag = 0xFF;
        let before = consumer_state(&sim);
        sim.pending_item = 0;
        sim.item_consumer();
        assert_eq!(sim.pending_item, 0);
        assert_eq!(consumer_state(&sim), before);
    }

    /// Every id above the table falls straight to `$3539`: the byte clears
    /// and nothing else changes.
    #[test]
    fn unmatched_item_ids_clear_without_effects() {
        for id in [10u8, 0x20, 0x7F] {
            let mut sim = PlayerSim::new();
            let before = consumer_state(&sim);
            sim.pending_item = id;
            sim.item_consumer();
            assert_eq!(sim.pending_item, 0, "id {id:#04X}");
            assert_eq!(consumer_state(&sim), before, "id {id:#04X}");
        }
    }

    /// `$3456` (id 1): mushroom — tier 1, `sSfxRequest1 = 20` (the engine
    /// does *not* write sSfxRequest0 here), and `$3534`'s 64-frame
    /// invincibility.
    #[test]
    fn mushroom_pickup_grants_tier1_with_the_3534_timer() {
        let mut sim = PlayerSim::new();
        sim.powerup = 0;
        sim.sfx0 = 3; // must survive: id 1 writes sSfxRequest1
        sim.pending_item = 1;
        sim.item_consumer();
        assert_eq!(sim.powerup, 1);
        assert_eq!(sim.invincibility, 64);
        assert_eq!(sim.sfx1, 20);
        assert_eq!(sim.sfx0, 3);
        assert_eq!(sim.pending_item, 0);
    }

    /// `$3467` (id 2): the fire flower is tier 3.
    #[test]
    fn fire_flower_pickup_grants_tier3() {
        let mut sim = PlayerSim::new();
        sim.powerup = 0;
        sim.pending_item = 2;
        sim.item_consumer();
        assert_eq!(sim.powerup, 3);
        assert_eq!(sim.invincibility, 64);
        assert_eq!(sim.sfx1, 20);
        assert_eq!(sim.sfx0, 0);
        assert_eq!(sim.pending_item, 0);
    }

    /// `$3478` (id 3): the carrot is the bunny tier 2.
    #[test]
    fn carrot_pickup_grants_tier2() {
        let mut sim = PlayerSim::new();
        sim.powerup = 0;
        sim.pending_item = 3;
        sim.item_consumer();
        assert_eq!(sim.powerup, 2);
        assert_eq!(sim.invincibility, 64);
        assert_eq!(sim.sfx1, 20);
        assert_eq!(sim.pending_item, 0);
    }

    /// `$3489` (id 4): timers 4/64 and `sMusicRequest = 4`; the star never
    /// touches the invincibility blink.
    #[test]
    fn star_pickup_sets_the_timers_and_the_music_request() {
        let mut sim = PlayerSim::new();
        sim.invincibility = 128;
        sim.pending_item = 4;
        sim.item_consumer();
        assert_eq!((sim.star_high, sim.star_low), (4, 64));
        assert_eq!(sim.music_request, 4);
        assert_eq!(sim.invincibility, 128);
        assert_eq!(sim.pending_item, 0);
    }

    /// The star's `sStompChainCount` clear is conditional —
    /// `(sStarTimerLow ^ $FF) | sStarTimerHigh == 0` — so only the exhausted
    /// (high 0, low `$FF`) pair the `$5D39` decrement stops at resets the
    /// chain. A timer that never ran (0, 0) or one still counting leaves it
    /// alone.
    #[test]
    fn star_pickup_resets_the_stomp_chain_only_on_an_expired_timer() {
        for (high, low) in [
            (0u8, 0u8),
            (0, 1),
            (0, 0xFE),
            (1, 0xFF),
            (4, 0x40),
            (0, 0xFF),
        ] {
            let mut sim = PlayerSim::new();
            sim.star_high = high;
            sim.star_low = low;
            sim.pending_item = 4;
            sim.item_consumer();
            let expected = high == 0 && low == 0xFF;
            assert_eq!(
                sim.stomp_chain_reset, expected,
                "timer ({high}, {low:#04X})"
            );
            // The runtime drains the flag — it is a one-shot.
            assert_eq!(sim.take_stomp_chain_reset(), expected);
            assert!(!sim.take_stomp_chain_reset());
        }
    }

    /// `$34AF` (id 5): `inc a / daa` on the BCD lives byte, clamped at 99
    /// when the increment wraps.
    #[test]
    fn one_up_heart_bcd_increments_lives_up_to_99() {
        for (before, after) in [
            (0x00u8, 0x01u8),
            (0x05, 0x06),
            (0x09, 0x10),
            (0x98, 0x99),
            (0x99, 0x99),
        ] {
            let mut sim = PlayerSim::new();
            sim.lives = before;
            sim.pending_item = 5;
            sim.item_consumer();
            assert_eq!(sim.lives, after, "lives {before:#04X} + 1");
            assert_eq!(sim.sfx0, 9);
            assert_eq!(sim.pending_item, 0);
        }
    }

    /// `$34EC` (id 6): sfx0 12, `sGoalBell` set (the port's bool is compared
    /// as 12), the low-time latch cleared, and vvel lifted to 24 only when
    /// it was below `$18` — `$18` and above stay put.
    #[test]
    fn bell_pickup_clears_the_low_time_latch_and_lifts_vvel_to_24() {
        for (before, after) in [
            (0u8, 24u8),
            (0x17, 24),
            (0x18, 0x18),
            (0x33, 0x33),
            (0xFF, 0xFF),
        ] {
            let mut sim = PlayerSim::new();
            sim.vvel = before;
            sim.low_time_flag = 0xFF;
            sim.pending_item = 6;
            sim.item_consumer();
            assert_eq!(sim.vvel, after, "vvel {before:#04X}");
            assert!(sim.goal_bell);
            assert_eq!(sim.low_time_flag, 0);
            assert_eq!(sim.sfx0, 12);
            assert_eq!(sim.pending_item, 0);
        }
    }

    /// `$34C9` (id 7): the secret-route flag — the one transition the gate
    /// captures actually replay (traj13/traj14's `secret` check).
    #[test]
    fn secret_route_pickup_sets_the_flag() {
        let mut sim = PlayerSim::new();
        sim.pending_item = 7;
        sim.item_consumer();
        assert_eq!(sim.secret_route, 0xFF);
        assert_eq!(sim.sfx0, 21);
        assert_eq!(sim.pending_item, 0);
    }

    /// `$34D9` (id 8): `set 2` on `sMoonPhysics` — bit 2 only, other bits
    /// kept, and idempotent on a repeat pickup.
    #[test]
    fn moon_pickup_sets_the_physics_bit_and_keeps_the_rest() {
        let mut sim = PlayerSim::new();
        sim.moon = 0x03;
        sim.pending_item = 8;
        sim.item_consumer();
        assert_eq!(sim.moon, 0x07);
        assert_eq!(sim.sfx1, 20);
        assert_eq!(sim.sfx0, 0);
        assert_eq!(sim.pending_item, 0);
        sim.pending_item = 8;
        sim.item_consumer();
        assert_eq!(sim.moon, 0x07);
    }

    /// `$350A` (id 9): `add $50`/`daa` — fifty coins in BCD, the decimal
    /// carry into sCoinHigh, and the `cp $0A` clamp to 999.
    #[test]
    fn coin_bag_adds_50_in_bcd_with_carry_and_caps_at_999() {
        for ((hi, lo), (want_hi, want_lo)) in [
            ((0x00u8, 0x00u8), (0x00u8, 0x50u8)),
            ((0x00, 0x45), (0x00, 0x95)), // 45 + 50 = 95: no carry
            ((0x00, 0x49), (0x00, 0x99)), // 49 + 50 = 99: still no carry
            ((0x00, 0x50), (0x01, 0x00)), // 50 + 50 = 100: carry
            ((0x00, 0x99), (0x01, 0x49)), // 99 + 50 = 149
            ((0x08, 0x99), (0x09, 0x49)), // 899 + 50 = 949: high 9, no clamp
            ((0x09, 0x50), (0x09, 0x99)), // 950 + 50 = 1000 -> 999
            ((0x09, 0x99), (0x09, 0x99)), // already 999
        ] {
            let mut sim = PlayerSim::new();
            sim.coins_high = hi;
            sim.coins_low = lo;
            sim.pending_item = 9;
            sim.item_consumer();
            assert_eq!(
                (sim.coins_high, sim.coins_low),
                (want_hi, want_lo),
                "coins {hi:#04X}{lo:02X} + 50"
            );
            assert_eq!(sim.sfx0, 7);
            assert_eq!(sim.pending_item, 0);
        }
    }

    /// `step` runs the consumer at the update tail: the id staged by the
    /// *previous* frame's object pass lands and the byte clears.
    #[test]
    fn step_consumes_the_staged_item_at_the_update_tail() {
        let layout = blank_layout();
        let keys = PlayerKeys::default();
        let mut sim = small_sim();
        sim.pending_item = 7;
        sim.step(&keys, false, &layout);
        assert_eq!(sim.secret_route, 0xFF);
        assert_eq!(sim.sfx0, 21);
        assert_eq!(sim.pending_item, 0);
    }

    /// The tail order is `call $319F / call $3451`: a powerup-loss dispatch
    /// (tier down, 128-frame blink) is overwritten by the item the same
    /// frame — the mushroom wins, with the item's 64-frame timer.
    #[test]
    fn the_item_consumer_runs_after_the_death_dispatch() {
        let layout = blank_layout();
        let keys = PlayerKeys::default();
        let mut sim = PlayerSim::new(); // big
        sim.dead = 15; // a harmful contact this frame
        sim.pending_item = 1;
        sim.step(&keys, false, &layout);
        assert_eq!(sim.dead, 0); // $319F consumed the hit...
        assert_eq!(sim.powerup, 1); // ...and $3451's mushroom re-granted tier 1
        assert_eq!(sim.invincibility, 64); // $3534's 64, not the dispatch's 128
    }

    /// Travelling replaces the whole update (`$0FBB` calls `$10EC` and
    /// returns), so the tail — and with it the consumer — never runs: the
    /// staged id waits for the exit.
    #[test]
    fn a_travelling_frame_defers_the_pending_item() {
        let layout = blank_layout();
        let keys = PlayerKeys::default();
        let mut sim = small_sim();
        sim.pipe_dir = 0x10;
        sim.pending_item = 7;
        sim.step(&keys, false, &layout);
        assert_eq!(sim.pending_item, 7);
        assert_eq!(sim.secret_route, 0);
    }

    /// `$1DFE`'s probe height is `sCurPowerup`-dependent (`$1E17`: y+18 when
    /// nonzero, y+28 at 0) — a mushroom moves the ceiling sample up one row.
    /// (116, 218) is row 12 / col 6; (116, 228) is row 13 / col 6.
    #[test]
    fn mushroom_raises_the_ceiling_probe_from_y28_to_y18() {
        let mut layout = blank_layout();
        layout.cells[12 * 256 + 6] = 0x10; // solid only at the big-Mario sample
        let mut sim = small_sim(); // powerup 0
        assert!(!sim.ceiling_hit(&layout));
        assert_eq!(sim.ceiling_probe, (116, 228));
        sim.pending_item = 1;
        sim.item_consumer();
        assert_eq!(sim.powerup, 1);
        assert!(sim.ceiling_hit(&layout));
        assert_eq!(sim.ceiling_probe, (116, 218));
    }

    /// `$1E9C`: the y+20 side sample only exists while `sCurPowerup != 0`
    /// (and Mario is not ducking) — small Mario walks over a coin there, big
    /// Mario collects it. (120, 220) is row 12 / col 7.
    #[test]
    fn mushroom_enables_the_y20_wall_probe_and_its_coin_touch() {
        let mut layout = blank_layout();
        layout.cells[12 * 256 + 7] = 0x48; // a coin at the big-only sample
        let mut sim = small_sim();
        assert_eq!(sim.wall_tile(&layout, 120), None);
        assert_eq!(sim.coins_low, 0);
        assert!(sim.layout_writes.is_empty());
        sim.pending_item = 3; // the carrot raises the same probe
        sim.item_consumer();
        assert_eq!(sim.powerup, 2);
        assert_eq!(sim.wall_tile(&layout, 120), None); // a coin is not solid
        assert_eq!(sim.coins_low, 0x01); // but it is now collected
        assert_eq!(sim.layout_writes, vec![(cell_index(12, 7), 0x60)]);
    }

    /// The same y+20 sample becomes a *wall* once the powerup is nonzero:
    /// small Mario's shorter hitbox passes over the solid cell, tier 1 stops.
    #[test]
    fn mushroom_turns_the_y20_sample_into_a_wall() {
        let mut layout = blank_layout();
        layout.cells[12 * 256 + 7] = 0x10;
        let mut sim = small_sim();
        assert_eq!(sim.wall_tile(&layout, 120), None);
        sim.pending_item = 1;
        sim.item_consumer();
        assert_eq!(sim.wall_tile(&layout, 120), Some(0x10));
    }

    /// `$1C39`: the big-Mario hazard probe moves up to (x+16, y+22), so a
    /// `$50` only the tall hitbox reads kills after the powerup — and a
    /// fresh powerup grants the 64-frame blink the `$1C6D` guard reads.
    /// (116, 222) is row 12 / col 6.
    #[test]
    fn mushroom_raises_the_hazard_probe_from_y28_to_y22() {
        let mut layout = blank_layout();
        layout.cells[12 * 256 + 6] = 0x50; // a spike at the big-only sample
        let mut sim = small_sim();
        assert_eq!(sim.hazard_probe(&layout), (false, None));
        sim.pending_item = 1;
        sim.item_consumer();
        assert_eq!(sim.powerup, 1);
        // The arm now hits the big-Mario sample — but the fresh mushroom's
        // 64-frame blink is exactly the `$1C6D` guard: it enters and writes
        // no death byte.
        assert_eq!(sim.hazard_probe(&layout), (true, None));
        sim.invincibility = 0; // once the blink lapses the spike kills
        assert_eq!(sim.hazard_probe(&layout), (true, Some(15)));
    }

    // -----------------------------------------------------------------
    // $2F72 / $19D2 — the below-hit pass's two halves
    // -----------------------------------------------------------------

    /// `$01` bonk: the spin flag, the sfx and the type-3 bump slot land
    /// immediately, but the coin waits on the object side's `$2F72` — the
    /// alloc arms the pass with the probe's low bytes and `resolve_bump_coin`
    /// is the `$19D2` tail that credits it once the special flag is known.
    #[test]
    fn the_01_bonk_defers_its_coin_to_the_below_hit_pass() {
        let mut sim = PlayerSim::new();
        sim.ceiling_probe = (360, 479); // (px, py): the traj15 $01 block
        sim.block_bonk(0x01);
        assert_eq!(sim.spin_jump_bounce, 0xFF);
        assert_eq!(sim.sfx0, 2);
        assert_eq!(sim.block_slots[0][0], 3, "the $5B06 bump slot");
        assert_eq!(sim.coins_low, 0x00, "the coin is deferred");
        assert!(sim.bump_coin_pending);
        assert_eq!(sim.below_bonk, Some((0xDF, 0x68))); // 479, 360 low bytes
        sim.resolve_bump_coin(false);
        assert_eq!(sim.coins_low, 0x01);
        assert_eq!(sim.sfx0, 7, "$591F's credit sfx");
        assert_eq!(sim.block_slots[1][0], 4, "the coin-pop slot");
        assert_eq!(sim.object_bump_special, 0);
        assert!(!sim.bump_coin_pending);
    }

    /// The pass's `sObjectBumpSpecial` (a type-`$80` pose inside the bonk
    /// window) withholds the coin *and* the type-4 slot — the ROM's
    /// `and a / ret nz` ahead of `$2C77`.
    #[test]
    fn a_pose_in_the_bonk_window_withholds_the_coin() {
        let mut sim = PlayerSim::new();
        sim.ceiling_probe = (360, 479);
        sim.block_bonk(0x01);
        sim.resolve_bump_coin(true);
        assert_eq!(sim.coins_low, 0x00, "no credit");
        assert_eq!(sim.sfx0, 2, "and no $591F sfx");
        assert_eq!(sim.block_slots[0][0], 3, "the bump itself still ran");
        assert_eq!(sim.block_slots[1][0], 0, "no coin slot");
        assert_eq!(sim.object_bump_special, 0xFF);
        assert!(!sim.bump_coin_pending);
    }

    /// Only the `$01` bonk owes a coin: the other classes arm the pass but
    /// `resolve_bump_coin` is a plain no-op for them, and a bump whose
    /// `$5960` walk found no free slot never runs `$2F72` at all.
    #[test]
    fn other_bump_classes_owe_no_coin_and_a_full_table_arms_nothing() {
        let mut sim = PlayerSim::new();
        sim.ceiling_probe = (360, 479);
        sim.block_bonk(0x7E); // type 3, no coin
        assert_eq!(sim.below_bonk, Some((0xDF, 0x68)));
        assert!(!sim.bump_coin_pending);
        sim.resolve_bump_coin(false);
        assert_eq!(sim.coins_low, 0);
        assert_eq!(sim.block_slots[0][0], 3);
        // The 8-slot table full: the alloc's `cp $00 / ret z` bails before
        // the tail, so nothing is armed.
        let mut sim = PlayerSim::new();
        sim.ceiling_probe = (360, 479);
        for s in sim.block_slots.iter_mut() {
            s[0] = 1;
        }
        sim.block_bonk(0x02);
        assert_eq!(sim.below_bonk, None);
        assert!(!sim.bump_coin_pending);
    }

    /// The level reload clears the bump scratch — including the pass's arming
    /// and its pending coin.
    #[test]
    fn respawn_clears_the_below_hit_scratch() {
        let mut sim = PlayerSim::new();
        sim.ceiling_probe = (360, 479);
        sim.block_bonk(0x01);
        assert!(sim.below_bonk.is_some() && sim.bump_coin_pending);
        sim.respawn(34, 468);
        assert_eq!(sim.below_bonk, None);
        assert!(!sim.bump_coin_pending);
        assert_eq!(sim.object_bump_special, 0);
        assert_eq!(sim.block_slots, [[0; 16]; 8]);
    }

    /// The `$2F72` hand-off reads the *previous* frame's scroll — it runs in
    /// `$0F71`, before `$0B2A`/`$2062`. Pinned here as the exact byte math the
    /// replay loop feeds to `ObjectSim::below_bonk`: the traj15 bonk (camera
    /// 448/377, probe 360/479) lands on the block's centre (248, 151).
    #[test]
    fn the_bonk_point_maps_to_the_block_centre_in_screen_bytes() {
        let cam_x = 448u16;
        let cam_y = 377u16;
        let (px, py) = (360u16, 479u16);
        let scroll_y = cam_y.wrapping_sub(72) as u8; // no pan in-level
        let scroll_x = cam_x.wrapping_sub(80) as u8;
        assert_eq!(scroll_y, 0x31); // 305 & $FF
        assert_eq!(scroll_x, 0x70); // 368 & $FF
        let by = (py as u8 & 0xF0)
            .wrapping_add(8)
            .wrapping_sub(scroll_y)
            .wrapping_sub(16);
        let bx = ((px as u8).wrapping_add(8) & 0xF0)
            .wrapping_sub(scroll_x)
            .wrapping_sub(8);
        assert_eq!((by, bx), (0x97, 0xF8), "the block's centre (151, 248)");
    }

    // -----------------------------------------------------------------
    // $126F / $1327 — the spin jump: the engage toggle, the landing bonk
    // ($1993 off the floor probe) and the bounce.
    //
    // No capture holds DOWN mid-air on an odd powerup, so the console-side
    // route never engages a spin jump in the suite; the branch is pinned
    // here instead (the wiring into the object engine's stomp dispatch is
    // `MarioContact.spin_jump`, fed from `sim.spin_jump` by the runtime).
    // -----------------------------------------------------------------

    /// `$126F`, through the real per-frame entry: DOWN held, odd powerup,
    /// airborne and no moon physics engages (`$FF`) with the engage-edge
    /// spin sfx 14 on the first frame only; releasing DOWN — or an even
    /// powerup (small/bunny) — clears it.
    #[test]
    fn spin_jump_engages_on_down_in_air_only_for_odd_powerups() {
        let layout = blank_layout();
        let down = PlayerKeys {
            down: true,
            ..Default::default()
        };
        let mut sim = PlayerSim::new(); // powerup 1 (big): odd
        sim.on_ground = false;
        sim.step(&down, false, &layout);
        assert_eq!(sim.spin_jump, 0xFF);
        assert_eq!(sim.sfx0, 14, "the engage-edge spin sfx");
        // Still spinning next frame: the sfx does not re-fire.
        sim.sfx0 = 0;
        sim.step(&down, false, &layout);
        assert_eq!(sim.spin_jump, 0xFF);
        assert_eq!(sim.sfx0, 0);
        // DOWN released: $129A clears it.
        sim.step(&PlayerKeys::default(), false, &layout);
        assert_eq!(sim.spin_jump, 0);
        // Small Mario (powerup 0, even — the bit-0 test) never engages.
        let mut sim = small_sim();
        sim.on_ground = false;
        sim.step(&down, false, &layout);
        assert_eq!(sim.spin_jump, 0);
        assert_eq!(sim.sfx0, 0);
    }

    /// `$1327`: spin active, but the landing tile is not a bump class —
    /// `$1993`'s `cp $04 / ret nc` leaves `sSpinJumpBounce` clear, so the
    /// ordinary `$134F` landing runs (on-ground set, vvel untouched).
    #[test]
    fn spin_jump_lands_normally_on_solid_ground() {
        let keys = PlayerKeys::default();
        for tile in [0x04, 0x0F] {
            let mut sim = small_sim();
            sim.spin_jump = 0xFF;
            sim.vvel = 10;
            sim.landing(tile, &keys);
            assert!(sim.on_ground, "tile ${tile:02X} lands");
            assert!(!sim.in_air);
            assert_eq!(sim.vvel, 10, "the bounce never touched vvel");
            assert_eq!(sim.spin_jump_bounce, 0x00);
            assert!(sim.block_slots.iter().all(|s| s[0] == 0));
            assert!(sim.below_bonk.is_none());
        }
    }

    /// `$1327` + `$1993`: small Mario spinning onto a `$00` brick bumps type
    /// 2 (`$19B0`'s small arm — the brick is only rattled) and bounces:
    /// in-air, vvel 48, or 75 while A is held (the `bit 0` read on
    /// hKeysHeld). The `$134F` landing is skipped, so on-ground stays clear.
    #[test]
    fn spin_jump_on_a_brick_bumps_type_2_and_bounces_for_small_mario() {
        let mut sim = small_sim(); // powerup 0
        sim.spin_jump = 0xFF;
        sim.vvel = 10;
        sim.landing(0x00, &PlayerKeys::default());
        assert_eq!(sim.block_slots[0][0], 2, "the $19B0 small bump");
        assert_eq!(sim.block_slots[0][1], 1, "a fresh slot's timer");
        assert_eq!(sim.spin_jump_bounce, 0xFF);
        assert!(sim.in_air);
        assert_eq!(sim.vvel, 48);
        assert!(!sim.on_ground, "the $134F landing must not run");
        assert_eq!(sim.sfx0, 2, "$19F1's bump sfx");
        assert_eq!(sim.sfx2, 0, "small Mario destroys nothing");
        // The higher bounce: A held at the landing.
        let keys_a = PlayerKeys {
            a: true,
            ..Default::default()
        };
        let mut sim = small_sim();
        sim.spin_jump = 0xFF;
        sim.vvel = 10;
        sim.landing(0x00, &keys_a);
        assert_eq!(sim.block_slots[0][0], 2);
        assert_eq!(sim.spin_jump_bounce, 0xFF);
        assert_eq!(sim.vvel, 75);
        assert!(!sim.on_ground);
    }

    /// `$1327` + `$19B0`: big Mario spinning onto a `$00` brick destroys it —
    /// `sSfxRequest2` (sfx2) and the type-1 bump whose `$5CF1` write turns
    /// the cell into the `$60` empty metatile once the slot ticks — and
    /// bounces exactly like the small case.
    #[test]
    fn spin_jump_breaks_a_brick_for_big_mario_with_sfx2() {
        let mut sim = PlayerSim::new(); // powerup 1: big
        sim.x = 100;
        sim.y = 200;
        sim.spin_jump = 0xFF;
        sim.vvel = 10;
        sim.ceiling_probe = (360, 479); // slot Y/X round to cell (28, 22)
        sim.landing(0x00, &PlayerKeys::default());
        assert_eq!(sim.block_slots[0][0], 1, "the $19B0 destruction bump");
        assert_eq!(sim.sfx2, 1, "the break sound");
        assert_eq!(sim.spin_jump_bounce, 0xFF);
        assert!(sim.in_air);
        assert_eq!(sim.vvel, 48);
        assert!(!sim.on_ground);
        // The destruction shows when the slot ticks: timer 2 writes $60.
        sim.block_update();
        assert_eq!(sim.layout_writes, vec![(cell_index(28, 22), 0x60)]);
    }

    /// Without a spin, the landing never calls `$1993`: a `$00` brick
    /// underfoot is inert and Mario lands on it like any solid ground.
    #[test]
    fn normal_landing_never_bonks_the_tile_below() {
        let mut sim = small_sim(); // spin_jump 0
        sim.vvel = 10;
        sim.landing(0x00, &PlayerKeys::default());
        assert!(sim.on_ground);
        assert_eq!(sim.spin_jump_bounce, 0);
        assert!(sim.block_slots.iter().all(|s| s[0] == 0));
        assert_eq!(sim.sfx0, 0);
        assert_eq!(sim.sfx2, 0);
        assert_eq!(sim.vvel, 10, "vvel is not a bounce's");
    }

    /// The whole `$129F`→`$1327` path through `step`: a spinning big Mario
    /// falling onto a `$00` floor brick bumps it, bounces with the A-held 75
    /// — this frame's own gravity already decremented it once, and the hold
    /// suppresses the jump-cut — and never touches down (`$134F`'s
    /// on-ground write is skipped). The floor probe reads (x+12, y+44) =
    /// (112, 240) = row 14, col 6; camx is pinned so `$0FBF` does not wrap.
    #[test]
    fn step_spin_landing_bounces_off_the_brick() {
        let mut layout = blank_layout();
        layout.cells[14 * 256 + 6] = 0x00;
        let mut sim = PlayerSim::new(); // big
        sim.x = 100;
        sim.y = 196;
        sim.camx = 100;
        sim.vvel = 10; // falling: the $129F gate is open
        sim.spin_jump = 0xFF;
        let keys = PlayerKeys {
            down: true,
            a: true,
            ..Default::default()
        };
        sim.step(&keys, false, &layout);
        assert_eq!(sim.collision_tile, 0x00);
        assert_eq!(sim.block_slots[0][0], 1, "the brick was destroyed");
        assert_eq!(sim.sfx2, 1);
        assert_eq!(sim.spin_jump_bounce, 0xFF);
        assert!(sim.in_air);
        assert!(!sim.on_ground, "$134F must not run on the bounce");
        assert_eq!(sim.vvel, 74, "the 75 bounce minus gravity's decrement");
    }

    // -----------------------------------------------------------------
    // $311A / $27AF / $287E — the zero-lives game over.
    //
    // No capture reaches zero lives (every gate capture survives), so the
    // whole path is pinned here: the `sub 1 / daa` borrow that selects state
    // `$05` (and the tens that must *not* select it), `$27AF`'s state writes,
    // and `$287E`'s 255-frame fade + START wait.
    // -----------------------------------------------------------------

    /// The borrow is `sLives == 0`: `sub 1 / daa` on BCD 00 gives $99 with
    /// carry, which is `$314B`'s `ld a, 5 / ldh [$FF9B], a`.
    #[test]
    fn death_fade_borrow_on_zero_lives_raises_the_game_over_flag() {
        let mut sim = PlayerSim::new();
        sim.lives = 0;
        sim.fade_timer = 1;
        sim.fade_step();
        assert_eq!(sim.lives, 0x99);
        assert!(sim.game_over, "the borrow selects state $05");
    }

    /// `daa`'s low-nibble correction (0x06, taken when H is set) does not
    /// touch C, so a ten decrements to $09 and the game continues.
    #[test]
    fn death_fade_on_a_ten_lives_decrements_without_ending_the_game() {
        let mut sim = PlayerSim::new();
        sim.lives = 0x10;
        sim.fade_timer = 1;
        sim.fade_step();
        assert_eq!(sim.lives, 0x09);
        assert!(!sim.game_over, "$10 -> $09 carries nothing into the branch");
    }

    #[test]
    fn death_fade_on_a_plain_life_still_decrements_by_one() {
        let mut sim = PlayerSim::new();
        sim.lives = 0x05;
        sim.fade_timer = 1;
        sim.fade_step();
        assert_eq!(sim.lives, 0x04);
        assert!(!sim.game_over);
    }

    /// `$27AF`: lives back to 5, the coin counters cleared, the 255-frame
    /// fade armed, the music request set before the `$07` hand-off.
    #[test]
    fn game_over_enter_writes_the_05_fields() {
        let mut sim = PlayerSim::new();
        sim.lives = 0x99;
        sim.coins_low = 0x42;
        sim.coins_high = 0x01;
        sim.fade_timer = 0;
        sim.game_over_enter();
        assert_eq!(sim.lives, 5);
        assert_eq!((sim.coins_high, sim.coins_low), (0, 0));
        assert_eq!(sim.fade_timer, 255);
        assert_eq!(sim.music_request, 7);
    }

    /// `$287E`: the fade ticks down for 255 frames, then the screen waits for
    /// START, which hands back to state `$00` with `sMusicRequest = $FF`.
    #[test]
    fn game_over_step_ticks_the_fade_then_waits_for_start() {
        let mut sim = PlayerSim::new();
        sim.lives = 0;
        sim.fade_timer = 1;
        sim.fade_step();
        sim.game_over_enter();
        // The fade runs out over 255 frames; START during it does nothing.
        assert!(
            !sim.game_over_step(true),
            "START on the fade's first frame is ignored"
        );
        for _ in 0..254 {
            assert!(!sim.game_over_step(false));
        }
        assert_eq!(sim.fade_timer, 0);
        assert!(!sim.game_over_step(false), "no START, no hand-off");
        assert_eq!(sim.fade_timer, 0);
        assert!(sim.game_over_step(true), "the $00 hand-off frame");
        assert!(!sim.game_over);
        assert_eq!(sim.music_request, 0xFF);
        // The hand-off is one-shot: the phase is over.
        assert!(!sim.game_over_step(true));
    }

    /// Outside the game-over phase the poll is inert (the flag gates it), and
    /// a level reload clears the flag like the other phase flags.
    #[test]
    fn game_over_step_needs_the_phase_and_respawn_clears_it() {
        let mut sim = PlayerSim::new();
        sim.fade_timer = 0;
        assert!(!sim.game_over_step(true));
        sim.game_over = true;
        sim.respawn(112, 240);
        assert!(!sim.game_over);
    }

    // -----------------------------------------------------------------
    // $273E / $2756 — the START pause and its palette flash.
    //
    // No capture carries START (the replay `Frame` has no field for it), so
    // the handler is pinned here; what the gate cannot cover is flagged in the
    // docs rather than assumed.
    // -----------------------------------------------------------------

    /// The flash is bit 4 of the `$FF97` counter read every frame, not a timer
    /// with a reload: 16 frames of `$E7`, then 16 of `$93`, repeating.
    #[test]
    fn pause_flash_comes_off_bit_4_of_the_frame_counter() {
        let mut sim = PlayerSim::new();
        let seen: Vec<u8> = (0..32u8)
            .map(|i| {
                sim.frame_counter = i;
                sim.pause_flash()
            })
            .collect();
        assert_eq!(seen[..16], [0xE7; 16], "bit 4 clear -> $E7");
        assert_eq!(seen[16..], [0x93; 16], "bit 4 set -> $93");
        // It is the counter, not a toggle: the same value is back 16 later.
        sim.frame_counter = 32;
        assert_eq!(sim.pause_flash(), 0xE7);
        sim.frame_counter = 0x30;
        assert_eq!(sim.pause_flash(), 0x93);
    }

    /// `$273E` enters and `$2756` leaves; the flag *is* the port's model of
    /// `$FF9B = 8`.
    #[test]
    fn pause_enters_and_exits() {
        let mut sim = PlayerSim::new();
        assert!(!sim.paused);
        sim.pause_enter();
        assert!(sim.paused);
        sim.pause_exit();
        assert!(!sim.paused);
    }

    /// The SELECT branch needs `sLevelProgressFlags` bit 7 — the ROM's
    /// `and a / ret z` shape — and with it the level is abandoned through the
    /// `$14` hand-off rather than resumed.
    #[test]
    fn pause_select_exit_needs_the_cleared_level_flag() {
        let mut sim = PlayerSim::new();
        sim.pause_enter();
        sim.exit_tile_flag = 0xFF;
        sim.tile_anim_config = 0x11;
        assert!(
            !sim.pause_select_exit(),
            "bit 7 clear: SELECT leaves the handler alone"
        );
        assert!(sim.paused, "and the pause stands");

        sim.level_progress_flags = 0x80;
        assert!(sim.pause_select_exit(), "bit 7 set: the level is abandoned");
        assert!(!sim.paused);
        assert_eq!(sim.exit_tile_flag, 0);
        assert_eq!(sim.tile_anim_config, 0);
        assert!(sim.exit_completed, "the $14 hand-off is marked reached");
    }

    /// A level reload clears the pause like the other phase flags.
    #[test]
    fn respawn_clears_the_pause() {
        let mut sim = PlayerSim::new();
        sim.pause_enter();
        sim.respawn(112, 240);
        assert!(!sim.paused);
    }
}
