//! Faithful port of the camera/scroll update (`home.asm $0B2A`, called from
//! the in-level frame loop `$05E4` right after the player update `$0F71`).
//!
//! Model (free-scroll path; level 0 has no edge locks or autoscroll):
//!
//! - Horizontal: the player update publishes its speed (`$14BF`: `XSub` <
//!   `$80` → `sScrollDeltaRight`, else `sScrollDeltaLeft` = −XSub). The
//!   camera applies it when the matching enable is set, where the enables
//!   are set by the *draw* step (bank-1 `$54A7`): Mario's screen X (`mx −
//!   cam + 96`) ≥ `$60` sets right, < `$60` sets left; applying a delta
//!   clears the opposite enable. After applying, the camera nudges ±1 to
//!   park Mario at screen X `$3F`/`$40` when scrolling right, `$70`/`$71`
//!   when scrolling left.
//! - Vertical: delta = the player's Y change this frame (8-bit, against
//!   `sPrevMarioY`). Moving down scrolls only when Mario's screen Y ≥ `$50`
//!   (`$58` where the cell's `sScreenEdgeMap` bit3 — the hard down edge — is
//!   set); moving up only when < `$28` (`$40` when `sMapScrollLimit` is set,
//!   `$38` with bit2). No lead correction. The `$C0`/`$48` lock rows arm the
//!   screen transitions (`$0DED` scroll, flip-screen animation), which the
//!   `transition` field models.
//! - `$FF98` (`$0B39`): the cell's edge bits come from the level's 48-byte
//!   `sScreenEdgeMap` at `(Yhi & $0F) * 16 + (Xhi & $0F)`, read *before* the
//!   camera moves. Both follow axes test those bits, so a level whose table
//!   is not loaded scrolls with every edge clear — down 8 px early on each
//!   descent, which ratchets the camera below the level.
//! - `$307D` (the crouch-look pan) also lives here even though it is
//!   `$0F71`'s first call, not part of `$0B2A`: it needs `&mut` camera state.
//!   See [`CameraSim::crouch_pan`] — the frame loop calls it at the top of the
//!   player update, before `step`.
//! - All arithmetic on the low bytes like the engine; the hi byte is masked
//!   to `$0F` (12-bit world).

use crate::player_sim::{PlayerKeys, PlayerSim};

#[derive(Debug, Clone)]
pub struct CameraSim {
    pub x: u16,
    pub y: u16,
    pub right_enabled: bool,
    pub left_enabled: bool,
    pub map_scroll_limit: bool,
    pub map_scroll_flags: u8,
    pub prev_mario_y_lo: u8,
    /// `sScreenEdgeMap[y_hi * 16 + x_hi]` — per-16-cell-sector scroll locks
    /// (bit0 left edge `$B0`, bit1 `$50`, bit2 up `$48`, bit3 down `$C0`);
    /// zero for free-scroll levels.
    pub edge_map: [u8; 48],
    /// Replay/dev override: when set, `edge_bits()` returns this instead of
    /// the position-indexed lookup (the captured engine value of `$FF98`).
    pub edge_now: Option<u8>,
    pub auto_scroll: u8,
    /// `sScreenTransition` ($A20E): 0 none, else bit0 right / bit1 left /
    /// bit2 up / bit3 down. While nonzero the camera update is *only* the
    /// `$0DED` scroll handler and the frame loop skips the player update.
    pub transition: u8,
    /// `sSceneState` ($A28B) — nonzero high nibble parks the camera.
    pub scene_state: u8,
    /// `sBossDefeated` — a defeated boss freezes the camera until cleared.
    pub boss_defeated: u8,
    /// `sSceneMap` ($A900, 48 bytes = 3 rows x 16) for the `$0F0B` lookup.
    pub scene_map: Vec<u8>,
    /// `sSceneId` / `sMapEventFlag` / `sPaletteFadeStep` written by `$0F0B`.
    pub scene_id: u8,
    pub map_event_flag: u8,
    pub palette_fade_step: u8,
    /// `sSfxRequest0` set by the transition finish (fidelity only).
    pub sfx0: u8,
}

impl CameraSim {
    pub fn new(x: u16, y: u16) -> Self {
        Self {
            x,
            y,
            right_enabled: false,
            left_enabled: false,
            map_scroll_limit: false,
            map_scroll_flags: 0,
            prev_mario_y_lo: y as u8, // caller seeds with the live value
            edge_map: [0; 48],
            edge_now: None,
            auto_scroll: 0,
            transition: 0,
            scene_state: 0,
            boss_defeated: 0,
            scene_map: Vec::new(),
            scene_id: 0,
            map_event_flag: 0,
            palette_fade_step: 0,
            sfx0: 0,
        }
    }

    /// The level-(re)load camera reset: back to the header's start position,
    /// with the previous-Y latch at Mario's respawn Y. The scroll enables,
    /// the edge map and the level-constant flags are left alone.
    pub fn reset_level_start(&mut self, x: u16, y: u16, mario_y_lo: u8) {
        self.x = x;
        self.y = y;
        self.prev_mario_y_lo = mario_y_lo;
        self.transition = 0;
        self.edge_now = None;
        self.map_scroll_flags = 0;
        self.scene_state = 0;
        self.boss_defeated = 0;
        self.scene_id = 0;
        self.map_event_flag = 0;
        self.palette_fade_step = 0;
        self.sfx0 = 0;
    }

    /// `$0B39`: `sScreenEdgeMap[(sCameraY_hi << 4) | sCameraX_hi]` → `$FF98`.
    /// The ROM builds the index with `swap a` on the camera Y *high* byte
    /// (`$FFC9`) and `or`s the camera X high byte (`$FFCB`) — i.e.
    /// `(Yhi & $0F) * 16 + (Xhi & $0F)`, so the low nibble is the X cell, not
    /// the X low byte. The table is 48 bytes (3 rows of 16 cells), so a row
    /// above 2 reads 0; no level is taller than that.
    fn edge_bits(&self) -> u8 {
        if let Some(e) = self.edge_now {
            return e;
        }
        let xi = ((self.x >> 8) & 0x0F) as usize;
        let yi = ((self.y >> 8) & 0x0F) as usize;
        self.edge_map.get(yi * 16 + xi).copied().unwrap_or(0)
    }

    /// The `$FF98` this camera position derives — what the next camera update
    /// reads. The replay harness checks it against the capture's sampled byte
    /// (the console's `$FF98` still holds the *pre-move* cell's value when the
    /// camera crosses a screen boundary, so the check runs before the step).
    pub fn edge_byte(&self) -> u8 {
        self.edge_bits()
    }

    /// `$307D`: the crouch-look pan. This is the FIRST call of the player
    /// update `$0F71` (`home.asm:2102`), so the frame loop runs it at the top
    /// of the player update — before the player body and before this module's
    /// `step` (`$0B2A`). It reads the same `PlayerKeys` the caller hands to
    /// `PlayerSim::step` (the sim keeps no copy of the held buttons).
    ///
    /// The trigger is `hKeysHeld` (`$FF80`) tested with `cp` — an EXACT byte
    /// equality, not a mask. `ReadJoypad` (`$1FD2`) builds the byte as
    /// `(button nibble) | (direction nibble << 4)`, so the layout is
    ///
    /// ```text
    /// bit 0 A · bit 1 B · bit 2 Select · bit 3 Start
    /// bit 4 Right · bit 5 Left · bit 6 Up · bit 7 Down
    /// ```
    ///
    /// (cross-checks: the soft-reset test at `$0275` is `and $0F / cp $0F`
    /// for A+B+Start+Select; `$1189`/`$11C0` test `bit 7` for a DOWN pipe and
    /// `bit 6` for an UP pipe; `$101D`/`$134F` test `bit 0` for A.)
    ///
    /// So `$82` is exactly B+Down and `$42` exactly B+Up: holding *any* extra
    /// button — A, the opposite direction, left/right, Select/Start — makes
    /// the byte unequal and cancels the pan (including Up+Down together).
    /// `PlayerKeys` has no Select/Start, so they can never be "extra" here;
    /// every button it does model must be clear except the pair.
    ///
    /// Call position: at `$0F71`'s own position, i.e. *before*
    /// `PlayerSim::step`, so the `sMarioOnGround`/`sMarioOnObject` gate reads
    /// the flags the previous frame settled — the console sees them at the top
    /// of the player update, before this frame's landing/jump writes them.
    /// Running it after `PlayerSim::step` would pan one frame early on the
    /// landing frame and skip the jump frame (where the body clears
    /// `on_ground`). The frame loop skips `$0F71` wholesale while
    /// `sScreenTransition != 0` (`$05E4`), so callers must not call this
    /// during a transition.
    pub fn crouch_pan(&mut self, sim: &PlayerSim, keys: &PlayerKeys) {
        // $307D head: a running scene parks the pan (the *whole* byte, unlike
        // `step`, which only tests the high nibble).
        if self.scene_state != 0 {
            return;
        }
        // `sMarioOnGround | sMarioOnObject` — neither → skip.
        if !(sim.on_ground || sim.on_ground_obj) {
            return;
        }
        // The `cp $82` / `cp $42` tests, in that order. "Exactly B+Down" means
        // every other button PlayerKeys models is clear — the byte-equality
        // test, not a mask. Verified against `ReadJoypad` ($1FD2): hKeysHeld
        // packs directions in bits 4-7 (bit7 Down, bit6 Up, bit5 Left,
        // bit4 Right) and actions in bits 0-3 (bit1 B, bit0 A, bit2 Select,
        // bit3 Start), so $82 is exactly Down+B and $42 exactly Up+B.
        // Select and Start are the two bits `cp` also tests that PlayerKeys
        // cannot express today: holding either must CANCEL the pan. That is
        // latent until START is modelled (see the `pause_and_palette_flash`
        // queue item), at which point this gate has to include it.
        let b_down = keys.b && keys.down && !(keys.a || keys.up || keys.left || keys.right);
        let b_up = keys.b && keys.up && !(keys.a || keys.down || keys.left || keys.right);
        if b_down {
            self.pan_down();
        } else if b_up {
            self.pan_up();
        }
    }

    /// `sMarioScreenY` (`$A23B`) as this port models it — the draw step's
    /// `$FFC4 = marioY − camY + 98`, which `step` computes as `sy98` from the
    /// previous frame's Mario Y latch and the camera Y it is about to move.
    /// Both pan arms gate on it.
    fn screen_y98(&self) -> u8 {
        (self.prev_mario_y_lo.wrapping_sub(self.y as u8)).wrapping_add(98)
    }

    /// `$3098`: the B+Down arm. Needs `sMarioScreenY >= $20` (room below Mario
    /// on screen), then the cell's hard down edge (bit 3) blocks the pan at
    /// the `$C0` lock row. The scroll flag (bit 7) is set before that guard —
    /// a blocked pan still reports the down scroll.
    fn pan_down(&mut self) {
        if self.screen_y98() < 0x20 {
            return;
        }
        // The `$0B39` lookup, read at the *pre-move* cell. `$FF98` itself is
        // scratch: the camera step recomputes it at `$0B39` before anything
        // else reads it, so the port derives the byte here without storing it.
        let e = self.edge_bits();
        self.map_scroll_flags |= 0x80;
        if e & 8 != 0 && (self.y as u8) >= 0xC0 {
            return;
        }
        self.y = self.y.wrapping_add(1) & 0x0FFF;
    }

    /// `$30D9`: the B+Up arm. Needs `sMarioScreenY < $70`, then the cell's up
    /// edge (bit 2) blocks the pan below the `$48` lock row. Bit 6 of the
    /// scroll flags is set before that guard.
    fn pan_up(&mut self) {
        if self.screen_y98() >= 0x70 {
            return;
        }
        let e = self.edge_bits();
        self.map_scroll_flags |= 0x40;
        if e & 4 != 0 && (self.y as u8) < 0x49 {
            return;
        }
        self.y = self.y.wrapping_sub(1) & 0x0FFF;
    }

    /// The draw-step half (`$54A7`): Mario's screen X sets the enable for
    /// its side. Runs after the camera in the frame; call it once per step.
    pub fn update_enables(&mut self, mario_x: u16) {
        let sx = ((mario_x as u8).wrapping_sub(self.x as u8)).wrapping_add(96);
        if sx >= 0x60 {
            self.right_enabled = true;
        } else {
            self.left_enabled = true;
        }
    }

    /// One camera frame. `delta_right`/`delta_left` come from the player
    /// update (`$14BF`), `mario`/`mario_y_lo` from the post-player state.
    /// Takes `&mut PlayerSim` because the `$0DED` transition handler writes
    /// Mario's position directly (the frame loop skips the player update).
    pub fn step(&mut self, sim: &mut PlayerSim) {
        // $0B2A prologue: the scene machine / a defeated boss park the camera.
        if self.scene_state & 0xF0 != 0 {
            return;
        }
        if self.boss_defeated != 0 && self.boss_defeated & 0xF0 == 0 {
            return;
        }
        if self.transition != 0 {
            self.run_transition(sim);
            return;
        }
        // Deltas published by the player update ($14BF).
        let (mut d_r, mut d_l) = (0u8, 0u8);
        if sim.x_sub != 0 {
            if sim.x_sub < 0x80 {
                d_r = sim.x_sub;
            } else {
                d_l = 0u8.wrapping_sub(sim.x_sub);
            }
        }
        let mx_lo = sim.x as u8;
        let my_lo = sim.y as u8;
        let edges = self.edge_bits();

        // $0B5E right edge (bit0): at the $B0 column the transition fires
        // once Mario's screen X >= $A1; above it the camera pulls back; else
        // the normal right scroll. All paths then fall into $0BE5.
        if edges & 1 != 0 && (self.x as u8) == 0xB0 {
            let sx = (mx_lo.wrapping_sub(self.x as u8)).wrapping_add(96);
            if sx >= 0xA1 {
                self.transition = 1;
                self.scene_lookup(sim);
            }
        } else if edges & 1 != 0 && (self.x as u8) > 0xB0 {
            self.x = self.x.wrapping_sub(1);
            self.x &= 0x0FFF;
        } else if d_r != 0 && self.right_enabled {
            // $0B90: right scroll
            self.left_enabled = false;
            self.x = self.x.wrapping_add(d_r as u16);
            self.map_scroll_flags |= 0x10;
            let sx = (mx_lo.wrapping_sub(self.x as u8)).wrapping_add(96);
            if sx >= 0x40 {
                self.x = self.x.wrapping_add(1);
            } else if sx < 0x3F {
                self.x = self.x.wrapping_sub(1);
            }
            self.x &= 0x0FFF;
        }
        // $0BE5 left edge (bit1): at the $50 column the transition fires
        // once Mario's screen X < $0F (Mario warps to x=0, hi+1); below it a
        // pull-back; else the normal left scroll.
        if edges & 2 != 0 && (self.x as u8) == 0x50 {
            let sx = (mx_lo.wrapping_sub(self.x as u8)).wrapping_add(96);
            if sx < 0x0F {
                // MarioX = 0, MarioX_hi += 1 (mod $10) — $FFC2/$FFC3
                self.transition = 2;
                sim.x = (((sim.x >> 8).wrapping_add(1)) & 0x0F) << 8;
                self.scene_lookup(sim);
            }
        } else if edges & 2 != 0 && (self.x as u8) < 0x50 {
            // pull back toward the $50 lock column (no left scroll)
            self.x = self.x.wrapping_add(1);
            self.x &= 0x0FFF;
        } else if d_l != 0 && self.left_enabled {
            self.right_enabled = false;
            self.x = self.x.wrapping_sub(d_l as u16);
            self.map_scroll_flags |= 0x20;
            let sx = (mx_lo.wrapping_sub(self.x as u8)).wrapping_add(96);
            if sx < 0x70 {
                self.x = self.x.wrapping_sub(1);
            } else if sx >= 0x71 {
                self.x = self.x.wrapping_add(1);
            }
            self.x &= 0x0FFF;
        }

        // $0C7D: vertical follow. `$FF99` (the follow metric) is
        // `mario − cam + $60` at the current position; the *transition*
        // checks read `sMarioScreenY` ($A23B), which the frame loop writes
        // (`$54A7` flow) *before* the player update — so it holds
        // `prev_mario_y − cam + $62` from the check's point of view.
        let sy = (my_lo.wrapping_sub(self.y as u8)).wrapping_add(96);
        let sy98 = (self.prev_mario_y_lo.wrapping_sub(self.y as u8)).wrapping_add(98);
        let dv = my_lo.wrapping_sub(self.prev_mario_y_lo);
        if dv != 0 {
            if dv < 0x80 {
                // moved down: $0CEC
                self.map_scroll_flags |= 0x80;
                if edges & 8 != 0 {
                    if (self.y as u8) == 0xC0 {
                        // $C0 lock row: the down transition fires at
                        // sMarioScreenY >= $90.
                        if sy98 >= 0x90 {
                            self.transition = 8;
                            self.scene_lookup(sim);
                            // the engine then takes the up path ($0D05), whose
                            // gates always skip here (sy >= $90 > $28/$40) —
                            // so the camera does not move this frame — and
                            // sets the map-scroll flags bit6.
                            self.map_scroll_flags |= 0x40;
                        }
                    } else if (self.y as u8) < 0xC0 && sy >= 0x58 {
                        self.y = self.y.wrapping_add(dv as u16);
                    } else if (self.y as u8) > 0xC0 {
                        self.y = self.y.wrapping_sub(1);
                    }
                } else if sy >= 0x50 {
                    self.y = self.y.wrapping_add(dv as u16);
                }
            } else {
                // moved up: $0D05
                let d_u = 0u8.wrapping_sub(dv);
                self.map_scroll_flags |= 0x40;
                if edges & 4 != 0 {
                    if (self.y as u8) == 0x48 {
                        // $48 lock row: the exit-up transition fires at
                        // sMarioScreenY < $1B — Mario warps to Y=0 with the
                        // camera's hi.
                        if sy98 < 0x1B {
                            self.transition = 4;
                            sim.y = ((self.y >> 8) & 0x0F) << 8;
                            if self.scene_state & 0xF0 == 0 {
                                self.scene_lookup(sim);
                            }
                        }
                    } else if (self.y as u8) < 0x48 {
                        self.y = self.y.wrapping_add(1);
                    } else {
                        let thresh = if self.map_scroll_limit { 0x40 } else { 0x38 };
                        if sy < thresh {
                            self.y = self.y.wrapping_sub(d_u as u16);
                        }
                    }
                } else {
                    let thresh = if self.map_scroll_limit { 0x40 } else { 0x28 };
                    if sy < thresh {
                        self.y = self.y.wrapping_sub(d_u as u16);
                    }
                }
            }
        }
        self.y &= 0x0FFF;
        self.prev_mario_y_lo = my_lo;
    }

    /// `$0DED`: the screen-transition scroll. While `sScreenTransition != 0`
    /// the camera update *is* this — the frame loop also skips the player
    /// update, so the pipe travel pauses here and the handler walks Mario's
    /// position instead. Finishes at the target column/row low byte.
    fn run_transition(&mut self, sim: &mut PlayerSim) {
        let t = self.transition;
        if t & 1 != 0 {
            self.x = self.x.wrapping_add(4) & 0x0FFF;
            self.map_scroll_flags = 0x10;
            sim.walk_anim_tick(1);
            sim.x = sim.x.wrapping_add(1);
            if self.x as u8 == 0x50 {
                self.finish_transition();
            }
        } else if t & 2 != 0 {
            self.x = self.x.wrapping_sub(4) & 0x0FFF;
            self.map_scroll_flags = 0x20;
            sim.walk_anim_tick(1);
            sim.x = sim.x.wrapping_sub(1);
            if self.x as u8 == 0xB0 {
                self.finish_transition();
            }
        } else if t & 4 != 0 {
            self.y = self.y.wrapping_sub(4) & 0x0FFF;
            self.map_scroll_flags = 0x40;
            sim.walk_anim_tick(1);
            sim.y = sim.y.wrapping_sub(1);
            if self.y as u8 == 0xB8 {
                self.finish_transition();
            }
        } else if t & 8 != 0 {
            self.y = self.y.wrapping_add(4) & 0x0FFF;
            self.map_scroll_flags = 0x80;
            sim.walk_anim_tick(1);
            // the down scroll carries Mario +1, or +2 on odd `$FF97` frames
            let b = 1 + (sim.phase & 1);
            sim.y = sim.y.wrapping_add(b as u16);
            if self.y as u8 == 0x48 {
                self.finish_transition();
            }
        }
    }

    /// `$0EB0`: end the transition. The scene-load branch (`sSceneState &
    /// $0F != 0` → state 240 + bank-1 `$3C94` template loader) is NOT ported
    /// — it belongs to the scene system; level 1's scene map is all zeros, so
    /// its pipes keep `sSceneState` 0 and skip it.
    fn finish_transition(&mut self) {
        self.transition = 0;
        if self.boss_defeated & 0xF0 != 0 {
            self.boss_defeated &= 0x0F;
        }
        self.sfx0 = 5;
    }

    /// `$0F0B`: latch the scene id under Mario from `sSceneMap` ($A900).
    fn scene_lookup(&mut self, sim: &PlayerSim) {
        self.map_event_flag = 0xFF;
        let idx = (((sim.y >> 8) as usize & 0x0F) << 4) | ((sim.x >> 8) as usize & 0x0F);
        self.scene_id = self.scene_map.get(idx).copied().unwrap_or(0);
        self.palette_fade_step = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape of level 1's real `sScreenEdgeMap` record (bank 9, pointer
    /// `$4004`, `layout_index` 0): row 1 carries the up/down lock bits, the
    /// right column `$0D`, the last cell `$0F`.
    fn level1_table() -> [u8; 48] {
        let mut t = [0u8; 48];
        t[16] = 0x0E;
        t[17] = 0x0C;
        t[31] = 0x0D;
        t[46] = 0x0F;
        t
    }

    fn cam_at(x: u16, y: u16) -> CameraSim {
        let mut c = CameraSim::new(x, y);
        c.edge_map = level1_table();
        c
    }

    /// `$0B39`: the index is `(Yhi & $0F) * 16 + (Xhi & $0F)` — the X *cell*
    /// in the low nibble (the ROM `or`s the X high byte, not the low one).
    #[test]
    fn edge_byte_indexes_by_the_camera_cell() {
        assert_eq!(cam_at(80, 448).edge_byte(), 0x0E); // x cell 0, row 1
        assert_eq!(cam_at(257, 448).edge_byte(), 0x0C); // x cell 1
        assert_eq!(cam_at(3840, 448).edge_byte(), 0x0D); // x cell 15
        assert_eq!(cam_at(80, 720).edge_byte(), 0x00); // row 2, unset
    }

    /// The table is 48 bytes (3 rows of 16 cells): a row above 2 reads 0
    /// instead of wrapping into the next row.
    #[test]
    fn edge_byte_is_zero_past_the_third_row() {
        assert_eq!(cam_at(257, 448 + 4 * 256).edge_byte(), 0x00);
        assert_eq!(cam_at(257, 448 + 15 * 256).edge_byte(), 0x00);
    }

    /// The replay fallback: `edge_now` still overrides the lookup, which is
    /// what a level with no extracted table falls back to.
    #[test]
    fn edge_now_overrides_the_table() {
        let mut c = cam_at(257, 448);
        c.edge_now = Some(0xAB);
        assert_eq!(c.edge_byte(), 0xAB);
    }

    /// The drift bug: `$0C7D`'s down-follow waits for screen Y `$58` when the
    /// cell's hard down edge (bit3) is set, and only `$50` when it is clear.
    /// An all-zero table therefore scrolls the camera down 8 px early on every
    /// descent — and because each jump re-arms it, the camera ratchets below
    /// the level and the floor scrolls up into view.
    #[test]
    fn the_down_follow_waits_for_the_hard_edge_threshold() {
        // Camera at row 1, Mario 84 px down the screen: between $50 and $58.
        let mut sim = PlayerSim::new();
        sim.x = 100;
        sim.x_sub = 0;
        sim.y = 0x34;

        let mut with_edge = cam_at(80, 0x140);
        with_edge.prev_mario_y_lo = 0x30; // descended 4 px this frame
        with_edge.step(&mut sim);
        assert_eq!(with_edge.y, 0x140, "bit3 set: 84 < $58, the camera holds");

        let mut without_edge = cam_at(80, 0x140);
        without_edge.edge_map[16] = 0x04; // same cell, bit3 clear
        without_edge.prev_mario_y_lo = 0x30;
        without_edge.step(&mut sim);
        assert_eq!(without_edge.y, 0x144, "bit3 clear: 84 >= $50, it follows");
    }

    // ---- $307D: the crouch-look pan -------------------------------------

    /// A camera at row 1 (all-clear edge table), Mario latched level with it
    /// so `sMarioScreenY` = `$00 − $00 + $62` = `$62` — mid-screen, inside
    /// both pan gates.
    fn pan_cam(y: u16) -> CameraSim {
        let mut c = CameraSim::new(0, y);
        c.prev_mario_y_lo = y as u8;
        c
    }

    fn grounded() -> PlayerSim {
        let mut s = PlayerSim::new();
        s.on_ground = true;
        s
    }

    /// `hKeysHeld == $82`: B + Down, nothing else.
    fn b_down() -> PlayerKeys {
        PlayerKeys {
            down: true,
            b: true,
            ..Default::default()
        }
    }

    /// `hKeysHeld == $42`: B + Up, nothing else.
    fn b_up() -> PlayerKeys {
        PlayerKeys {
            up: true,
            b: true,
            ..Default::default()
        }
    }

    /// `$3098`: B+Down pans the camera down exactly 1 px and sets
    /// `sMapScrollFlags` bit 7.
    #[test]
    fn crouch_pan_down_moves_the_camera_one_pixel() {
        let mut c = pan_cam(0x140);
        c.crouch_pan(&grounded(), &b_down());
        assert_eq!(c.y, 0x141);
        assert_eq!(c.map_scroll_flags, 0x80, "bit7 = the down-scroll flag");
    }

    /// `$30D9`: B+Up pans the camera up exactly 1 px and sets bit 6.
    #[test]
    fn crouch_pan_up_moves_the_camera_one_pixel() {
        let mut c = pan_cam(0x140);
        c.crouch_pan(&grounded(), &b_up());
        assert_eq!(c.y, 0x13F);
        assert_eq!(c.map_scroll_flags, 0x40, "bit6 = the up-scroll flag");
    }

    /// `$3098`'s hard down edge: with bit3 set the pan is blocked once the
    /// camera's lo byte reaches the `$C0` lock row, so it cannot scroll past
    /// a locked screen boundary. One px earlier it still runs, and with bit3
    /// clear the row is no obstacle. The scroll flag is set *before* the
    /// guard, so a blocked pan still reports the down scroll.
    #[test]
    fn crouch_pan_down_stops_at_the_hard_down_edge() {
        let mut c = pan_cam(0x1C0);
        c.edge_map[16] = 0x08; // bit3 set in row 1, x cell 0
        c.crouch_pan(&grounded(), &b_down());
        assert_eq!(c.y, 0x1C0, "$C0 row + bit3: the pan is blocked");
        assert_eq!(c.map_scroll_flags, 0x80, "the flag is set before the guard");

        let mut c = pan_cam(0x1BF);
        c.edge_map[16] = 0x08;
        c.crouch_pan(&grounded(), &b_down());
        assert_eq!(c.y, 0x1C0, "below the lock row the pan still runs");

        let mut c = pan_cam(0x1C0);
        c.edge_map[16] = 0x04; // bit3 clear
        c.crouch_pan(&grounded(), &b_down());
        assert_eq!(c.y, 0x1C1, "no hard down edge: the row is no obstacle");
    }

    /// `$30D9`'s up edge: bit2 set blocks the pan while the camera's lo byte
    /// is below `$49` (the ROM returns for lo < `$49`), so the camera cannot
    /// scroll above the `$48` lock row. Bit 6 of the scroll flags is set
    /// before the guard.
    #[test]
    fn crouch_pan_up_stops_at_the_up_edge() {
        let mut c = pan_cam(0x140);
        c.edge_map[16] = 0x04; // bit2 set
        c.crouch_pan(&grounded(), &b_up());
        assert_eq!(c.y, 0x140, "bit2 + lo < $49: the pan is blocked");
        assert_eq!(c.map_scroll_flags, 0x40, "the flag is set before the guard");

        let mut c = pan_cam(0x149);
        c.edge_map[16] = 0x04;
        c.crouch_pan(&grounded(), &b_up());
        assert_eq!(c.y, 0x148, "at lo $49 the guard opens");

        let mut c = pan_cam(0x100);
        c.edge_map[16] = 0x08; // only the down edge
        c.crouch_pan(&grounded(), &b_up());
        assert_eq!(c.y, 0x0FF, "no up edge: the pan runs from anywhere");
    }

    /// `$3098`'s `sMarioScreenY` gate: `cp $20 / ret c`, so screen Y `$1F`
    /// blocks and `$20` passes. The gate precedes the flag set.
    #[test]
    fn crouch_pan_down_needs_screen_y_at_least_20() {
        let mut c = pan_cam(0x140);
        c.prev_mario_y_lo = 0xEE; // $EE − $40 + $62 = $10: near the screen top
        c.crouch_pan(&grounded(), &b_down());
        assert_eq!(c.y, 0x140);
        assert_eq!(c.map_scroll_flags, 0x00, "the gate precedes the flag set");

        let mut c = pan_cam(0x140);
        c.prev_mario_y_lo = 0xFE; // $FE − $40 + $62 = $20
        c.crouch_pan(&grounded(), &b_down());
        assert_eq!(c.y, 0x141, "$20 is allowed (`ret c` is < $20)");
    }

    /// `$30D9`'s gate: `cp $70 / ret nc`, so screen Y `$70` blocks and `$6F`
    /// passes. The gate precedes the flag set.
    #[test]
    fn crouch_pan_up_needs_screen_y_below_70() {
        let mut c = pan_cam(0x140);
        c.prev_mario_y_lo = 0x4E; // $4E − $40 + $62 = $70
        c.crouch_pan(&grounded(), &b_up());
        assert_eq!(c.y, 0x140);
        assert_eq!(c.map_scroll_flags, 0x00, "the gate precedes the flag set");

        let mut c = pan_cam(0x140);
        c.prev_mario_y_lo = 0x4D; // $4D − $40 + $62 = $6F
        c.crouch_pan(&grounded(), &b_up());
        assert_eq!(c.y, 0x13F);
    }

    /// `$307D`'s head: `sMarioOnGround | sMarioOnObject` must be nonzero —
    /// neither grounded nor on an object skips the pan, and standing on an
    /// object alone is enough.
    #[test]
    fn crouch_pan_needs_ground_or_an_object() {
        let mut airborne = grounded();
        airborne.on_ground = false;
        let mut c = pan_cam(0x140);
        c.crouch_pan(&airborne, &b_down());
        assert_eq!(c.y, 0x140, "airborne: no pan");
        assert_eq!(c.map_scroll_flags, 0x00);

        let mut on_obj = grounded();
        on_obj.on_ground = false;
        on_obj.on_ground_obj = true;
        let mut c = pan_cam(0x140);
        c.crouch_pan(&on_obj, &b_down());
        assert_eq!(c.y, 0x141, "on an object alone pans");
    }

    /// `cp $82` / `cp $42` are byte-equality tests on `hKeysHeld`, not masks:
    /// any extra held button — A, the opposite direction, left/right — makes
    /// the byte unequal and cancels the pan. (`$82` = B+Down, `$42` = B+Up in
    /// the `$1FD2` layout: bit0 A, bit1 B, bit2 Select, bit3 Start, bit4
    /// Right, bit5 Left, bit6 Up, bit7 Down.) `PlayerKeys` has no
    /// Select/Start, so those can never be the extra button here.
    #[test]
    fn crouch_pan_requires_exactly_b_plus_the_direction() {
        let not_b_down: [(&str, PlayerKeys); 6] = [
            (
                "B+Down+A ($83)",
                PlayerKeys {
                    down: true,
                    b: true,
                    a: true,
                    ..Default::default()
                },
            ),
            (
                "B+Down+Up ($C2)",
                PlayerKeys {
                    up: true,
                    down: true,
                    b: true,
                    ..Default::default()
                },
            ),
            (
                "B+Down+Right ($92)",
                PlayerKeys {
                    right: true,
                    down: true,
                    b: true,
                    ..Default::default()
                },
            ),
            (
                "B+Down+Left ($A2)",
                PlayerKeys {
                    left: true,
                    down: true,
                    b: true,
                    ..Default::default()
                },
            ),
            (
                "B alone ($02)",
                PlayerKeys {
                    b: true,
                    ..Default::default()
                },
            ),
            (
                "Down alone ($80)",
                PlayerKeys {
                    down: true,
                    ..Default::default()
                },
            ),
        ];
        for (what, k) in not_b_down {
            let mut c = pan_cam(0x140);
            c.crouch_pan(&grounded(), &k);
            assert_eq!(c.y, 0x140, "{what} must not pan down");
            assert_eq!(c.map_scroll_flags, 0x00, "{what} must not flag a scroll");
        }

        let mut c = pan_cam(0x140);
        c.crouch_pan(
            &grounded(),
            &PlayerKeys {
                up: true,
                b: true,
                right: true,
                ..Default::default()
            },
        );
        assert_eq!(c.y, 0x140, "B+Up+Right ($52) must not pan up");

        // The bare pairs still pan, so the extra-button rule is what blocked
        // the cases above.
        let mut c = pan_cam(0x140);
        c.crouch_pan(&grounded(), &b_down());
        assert_eq!(c.y, 0x141);
    }

    /// `$307D`: a running scene (`sSceneState != 0` — the whole byte, not just
    /// the high nibble `step` parks on) skips the pan entirely.
    #[test]
    fn crouch_pan_is_parked_by_a_running_scene() {
        for state in [1u8, 0x10, 0xF0] {
            let mut c = pan_cam(0x140);
            c.scene_state = state;
            c.crouch_pan(&grounded(), &b_down());
            assert_eq!(c.y, 0x140, "sSceneState ${state:02X}: no pan");
            assert_eq!(c.map_scroll_flags, 0x00);
        }
    }

    /// The edge byte is the `$0B39` lookup at the *pre-move* cell: at `$0FF`
    /// the camera is in row 0, and the `+1` carries it into row 1 — whose
    /// cell carries the hard down edge and would block. The row-0 cell
    /// decides, so the pan runs and crosses.
    #[test]
    fn crouch_pan_reads_the_edge_cell_before_it_moves() {
        let mut c = pan_cam(0x0FF);
        c.edge_map[0] = 0x00; // row 0, x cell 0
        c.edge_map[16] = 0x08; // row 1 carries the hard down edge
        c.prev_mario_y_lo = 0x00; // screen Y $00 − $FF + $62 = $63
        c.crouch_pan(&grounded(), &b_down());
        assert_eq!(c.y, 0x100, "the pre-move row-0 cell has no bit3");
    }

    /// The 16-bit add/sub: the `+1` carries and the `−1` borrows across the
    /// lo byte.
    #[test]
    fn crouch_pan_carries_across_the_lo_byte() {
        let mut c = pan_cam(0x1FF);
        c.prev_mario_y_lo = 0xFF; // screen Y $FF − $FF + $62 = $62
        c.crouch_pan(&grounded(), &b_down());
        assert_eq!(c.y, 0x200, "the +1 carries into the hi byte");

        let mut c = pan_cam(0x100);
        c.prev_mario_y_lo = 0x00; // screen Y $62
        c.crouch_pan(&grounded(), &b_up());
        assert_eq!(c.y, 0x0FF, "the −1 borrows from the hi byte");
    }

    /// The port's 12-bit world: a pan never leaves the hi byte wider than four
    /// bits (the ROM keeps `$FFC9` to four bits elsewhere in the camera code,
    /// and the `$0B39` index masks it anyway).
    #[test]
    fn crouch_pan_keeps_the_hi_byte_to_four_bits() {
        let mut c = pan_cam(0x0FFF);
        c.prev_mario_y_lo = 0xFF; // screen Y $62
        c.crouch_pan(&grounded(), &b_down());
        assert_eq!(c.y, 0x000, "$FFF + 1 wraps to $000");
    }

    /// The gate reads the draw step's `sMarioScreenY` — the previous frame's
    /// Mario Y latch (`prev_mario_y_lo`) against the camera — exactly like
    /// `step`'s `sy98`; the live `sim.y` is not what the pan gates on.
    #[test]
    fn crouch_pan_gates_on_the_screen_y_latch() {
        let mut sim = grounded();
        sim.y = 0x00;
        let mut c = pan_cam(0x140);
        c.prev_mario_y_lo = 0x4E; // screen Y $70 → blocked regardless of sim.y
        c.crouch_pan(&sim, &b_up());
        assert_eq!(c.y, 0x140);
    }
}
