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
    /// `sBossFightPhase` (`$A2B1`): the fight counter that `$8DFD` adds to the
    /// level's def base. Its only ROM writer is the scene script (`$3C000`'s
    /// per-screen handlers), which is unported, so it stays 0 here — which is
    /// also what every first-arrival fight uses.
    pub boss_fight_phase: u8,
    /// Set by `$0EB0`'s scene branch inside [`finish_transition`] and consumed
    /// by the frame loop (`take_blocked_scene_load`): the branch is *not*
    /// taken, because the handlers that would clear `$F0` are unported.
    ///
    /// [`finish_transition`]: CameraSim::finish_transition
    /// [`take_blocked_scene_load`]: CameraSim::take_blocked_scene_load
    pub scene_load_pending: bool,
    /// `sBossDefeated` — a defeated boss freezes the camera until cleared.
    pub boss_defeated: u8,
    /// `sSceneMap` ($A900, 48 bytes = 3 rows x 16) for the `$0F0B` lookup.
    pub scene_map: Vec<u8>,
    /// `sSceneId` / `sMapEventFlag` / `sPaletteFadeStep` written by `$0F0B`.
    pub scene_id: u8,
    pub map_event_flag: u8,
    pub palette_fade_step: u8,
    /// `sSceneTransitionFlag` / `sSceneTransitionLatch` / `sSceneDoneFlag`
    /// (`$A24A` / `$A249` / `$A2A8`) — the scene VM's bookkeeping. `$4x` sets
    /// the flag, `$21EE` latches it into the latch and zeroes the rest. Nothing
    /// else in this ROM reads them, so they are modelled for fidelity (and for
    /// the VM's tests), not for behaviour.
    pub scene_transition_flag: u8,
    pub scene_transition_latch: u8,
    pub scene_done_flag: u8,
    /// `sSfxRequest0` set by the transition finish (fidelity only).
    pub sfx0: u8,
    /// `sCameraPanTimer` ($A2B3): boss-transition bob timer.
    /// Decremented by `$54E9` during Mario's draw, before the object pass.
    pub pan_timer: u8,
    /// The pre-decrement phase latched by `$2062` for BG and level objects.
    pub scroll_pan: u8,
    /// The post-decrement phase subtracted from Mario's `$FFC4` by `$54E9`.
    pub player_pan: u8,
    /// `sScrollY` ($A2B0): latched scroll Y = `cam.y - 72 - (pan_timer & 3)`.
    pub scroll_y: u8,
    /// `sScrollX` ($A2B1): latched scroll X = `cam.x - 80`.
    pub scroll_x: u8,
    /// `sScrollDeltaRight` ($A20B): horizontal scroll delta accumulated for the camera pass.
    pub scroll_delta_right: u8,
    /// `sScrollDeltaLeft` ($A20C): horizontal scroll delta accumulated for the camera pass.
    pub scroll_delta_left: u8,
}

impl CameraSim {
    pub fn new(x: u16, y: u16) -> Self {
        let scroll_y = y.wrapping_sub(72) as u8;
        let scroll_x = x.wrapping_sub(80) as u8;
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
            boss_fight_phase: 0,
            scene_load_pending: false,
            boss_defeated: 0,
            scene_map: Vec::new(),
            scene_id: 0,
            map_event_flag: 0,
            palette_fade_step: 0,
            scene_transition_flag: 0,
            scene_transition_latch: 0,
            scene_done_flag: 0,
            sfx0: 0,
            pan_timer: 0,
            scroll_pan: 0,
            player_pan: 0,
            scroll_y,
            scroll_x,
            scroll_delta_right: 0,
            scroll_delta_left: 0,
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
        self.pan_timer = 0;
        self.player_pan = 0;
        self.scroll_delta_right = 0;
        self.scroll_delta_left = 0;
        self.update_scroll();
    }

    /// `$2062`: compute `sScrollY = camY - 72 - (sCameraPanTimer & 3)` and
    /// `sScrollX = camX - 80`.
    pub fn update_scroll(&mut self) {
        let b = (self.pan_timer & 3) as u16;
        self.scroll_pan = b as u8;
        self.scroll_y = self.y.wrapping_sub(72).wrapping_sub(b) as u8;
        self.scroll_x = self.x.wrapping_sub(80) as u8;
    }

    /// `$54E9`: decrement `sCameraPanTimer` if nonzero.
    pub fn tick_pan_timer(&mut self) {
        if self.pan_timer != 0 {
            self.pan_timer -= 1;
        }
    }

    /// `$060F` / `$0640`: latch scroll on a live in-level frame, then draw
    /// Mario (`$54E9`) before objects can write the next shake. Pause/fade
    /// states skip both calls. The dead draw (`$5550`) skips `$2062` and returns
    /// before `$54E9` when raw screen Y is in `$E0..=$FF`.
    pub fn update_player_pan(&mut self, mario_y: u16, dead: bool, in_level: bool) {
        if !in_level {
            return;
        }
        if !dead {
            // `$2062` is called even when `$0B2A` parks the camera for a boss.
            self.update_scroll();
        }
        let screen_y = (mario_y as u8).wrapping_sub(self.y as u8).wrapping_add(98);
        if dead && screen_y & 0xE0 == 0xE0 {
            self.player_pan = 0;
            return;
        }
        self.tick_pan_timer();
        self.player_pan = self.pan_timer & 3;
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
        // Select and Start are the two other bits `cp` tests, and PlayerKeys
        // now carries both (the pause work added them): holding either must
        // CANCEL the pan, so they are part of the exact-match test.
        let b_down = keys.b
            && keys.down
            && !(keys.a || keys.up || keys.left || keys.right || keys.start || keys.select);
        let b_up = keys.b
            && keys.up
            && !(keys.a || keys.down || keys.left || keys.right || keys.start || keys.select);
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

    /// `$0DAF`: the autoscroll horizontal path. `$0B2A` calls it *instead of*
    /// the whole right/left section when `sAutoScroll != 0`, then jumps
    /// straight to `$0C7D`, so the vertical follow below still runs.
    ///
    /// It does only three things: the right-edge transition trigger, a
    /// one-pixel-per-two-frames scroll, and `sScrollLeftEnabled` cleared with
    /// `sMapScrollFlags` bit 4 set. Unlike `$0B5E`/`$0BE5` it never applies the
    /// player-speed latches (`d_r`/`d_l`), never reads either enable flag, has
    /// no ±1 park correction, no `$FF99` pull-back, no left edge lock and no
    /// left transition. Mario is not driven by it at all — he keeps normal
    /// physics and drifts toward the left of the screen.
    fn run_autoscroll(&mut self, sim: &mut PlayerSim) {
        let cam_lo = self.x as u8;
        if self.edge_bits() & 1 != 0 && cam_lo >= 0xB0 {
            // The same shape as `$0B5E`, but `>=` where that one tests `==`,
            // and a Mario screen X below $A1 `ret`s outright — it does not
            // fall through to the scroll the way $0B5E's tail does.
            let sx = ((sim.x as u8).wrapping_sub(cam_lo)).wrapping_add(96);
            if sx < 0xA1 {
                return;
            }
            self.transition = 1;
            self.scene_lookup(sim);
            return;
        }
        // $0DCA: `ldh a,[$FF97] / and $01 / ret nz` — one px per TWO frames.
        if sim.phase & 1 != 0 {
            return;
        }
        self.left_enabled = false;
        // The carry lands in the camera's high byte, masked to four bits.
        self.x = self.x.wrapping_add(1) & 0x0FFF;
        self.map_scroll_flags |= 0x10;
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
            self.prev_mario_y_lo = sim.y as u8;
            self.update_scroll();
            return;
        }
        // Deltas published by the player update ($14BF) or platform carriers ($3CC0/$3CF1).
        let (mut d_r, mut d_l) = (self.scroll_delta_right, self.scroll_delta_left);
        if sim.x_sub != 0 {
            if sim.x_sub < 0x80 {
                d_r = sim.x_sub;
            } else {
                d_l = 0u8.wrapping_sub(sim.x_sub);
            }
        }
        self.scroll_delta_right = 0;
        self.scroll_delta_left = 0;
        let mx_lo = sim.x as u8;
        let my_lo = sim.y as u8;
        let edges = self.edge_bits();

        // $0B2A's dispatcher: with `sAutoScroll` set, `$0DAF` replaces the
        // ENTIRE horizontal section and then jumps straight to `$0C7D`, so
        // neither the right nor the left chain below runs on an autoscroll
        // frame.
        if self.auto_scroll != 0 {
            self.run_autoscroll(sim);
        } else {
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
        // `$0D97`: `ldh a,[$FFC0] / ld [sPrevMarioY],a` — the latch reads the
        // **live** Mario Y, not the value this body entered with. That matters
        // on the one frame the body itself moves Mario: the up-lock arm warps
        // `$FFC0 = 0` (`home.asm:1786`) before falling into this tail, so the
        // ROM latches 0 and the next frame's `dv` is measured from the warped
        // position. Latching the entry snapshot instead leaves `dv` short by the
        // pre-warp low byte for every frame after an up transition.
        self.prev_mario_y_lo = sim.y as u8;
        // $2062: update sScrollY and sScrollX immediately after camera step ($0B2A)
        self.update_scroll();
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
    /// $0F != 0` -> `sSceneState = $F0` -> `$3C94`) is **ported but split**: it
    /// arms the park here and the frame loop finishes it through
    /// [`begin_scene_load`], because `$0EB0` also writes `sMusicRequest`/
    /// `sLevelSong` and reads `sCurLevel` — which live with the player sim and
    /// the level, not the camera. `$F0` is what `$0652` reads to swap the
    /// frame's object pass for the boss driver (`$8DA9`), so the park and the
    /// driver must land in the same frame; see `ObjectSim::step`'s `boss_mode`.
    ///
    /// The `sBossDefeated` arm below is a SEPARATE `if` in the ROM
    /// (`$0EE4`): it runs whether or not the scene branch was taken, so it
    /// must not become an `else`.
    fn finish_transition(&mut self) {
        self.transition = 0;
        if self.scene_state & 0x0F != 0 {
            self.scene_load_pending = true;
        }
        if self.boss_defeated & 0xF0 != 0 {
            self.boss_defeated &= 0x0F;
        }
        self.sfx0 = 5;
    }

    /// `$0652`'s test: while `sSceneState & $F0 != 0` the frame's object pass is
    /// the boss driver (`$8DA9`) instead of `$8000`. Exposed as a method so the
    /// frame loops cannot re-derive it differently.
    pub fn boss_mode(&self) -> bool {
        self.scene_state & 0xF0 != 0
    }

    /// `$0EB0`'s arena entry: consume the transition request, park the camera
    /// at `$F0`, and return the boss music and level song. The runtime then
    /// calls `ObjectSim::spawn_boss` and switches to the boss driver.
    ///
    /// Callers check [`crate::scene::boss_def_portable`] before taking this
    /// branch. Definitions 0/1/2/3/4/5 finish through the reward's level-clear event;
    /// Wario definitions 6/7/8 finish through their phase/ending events.
    /// Unsupported definitions use `take_blocked_scene_load` instead.
    ///
    /// Returns `(sMusicRequest, sLevelSong)`: `$0ECA` writes both, and the
    /// level-`$18` arm (`$0ED9`) then overrides **only** `sMusicRequest`.
    pub fn begin_scene_load(&mut self, level_id: u8) -> Option<(u8, u8)> {
        if !self.scene_load_pending {
            return None;
        }
        self.scene_load_pending = false;
        self.scene_state = 0xF0;
        let level_song = 32;
        let music = if level_id == 0x18 {
            self.boss_fight_phase.wrapping_add(33)
        } else {
            level_song
        };
        Some((music, level_song))
    }

    pub fn take_blocked_scene_load(&mut self) -> bool {
        if !self.scene_load_pending {
            return false;
        }
        self.scene_load_pending = false;
        true
    }

    /// `$0F0B`: latch the scene id under Mario from `sSceneMap` ($A900).
    fn scene_lookup(&mut self, sim: &PlayerSim) {
        self.map_event_flag = 0xFF;
        let idx = (((sim.y >> 8) as usize & 0x0F) << 4) | ((sim.x >> 8) as usize & 0x0F);
        self.scene_id = self.scene_map.get(idx).copied().unwrap_or(0);
        self.palette_fade_step = 0;
    }

    /// `$21EE` — the script END epilogue, which `$20A4`'s no-scene path also
    /// jumps to, so it runs on every pass with no script latched (idempotently).
    /// It latches `sSceneTransitionFlag` into `sSceneTransitionLatch` and zeroes
    /// `sSceneId`, `sSceneIdHi`, the flag and `sSceneDoneFlag`. `sSceneIdHi`
    /// (`$A28F`) is never written by any code path in this ROM, so it has no
    /// field here.
    pub fn end_scene(&mut self) {
        self.scene_transition_latch = self.scene_transition_flag;
        self.scene_id = 0;
        self.scene_transition_flag = 0;
        self.scene_done_flag = 0;
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

    /// `$0EB0`'s branch is taken for an accepted arena definition:
    /// the park lands — which is what `$0652`
    /// reads — and `begin_scene_load` returns the song.
    #[test]
    fn the_scene_branch_is_taken_for_a_portable_def() {
        use crate::scene::boss_def_portable;
        assert!(boss_def_portable(1), "the door boss fight is ported");
        assert!(boss_def_portable(2), "the Mario Zone fight is ported");
        assert!(boss_def_portable(6), "Wario form 1 is ported");
        assert!(boss_def_portable(7), "Wario form 2 is ported");
        assert!(boss_def_portable(8), "Wario form 3 is ported");
        assert!(boss_def_portable(0), "the Tree Zone arena flow is verified");
        assert!(!boss_def_portable(9), "unknown definitions remain blocked");
        assert!(boss_def_portable(3), "definition 3 arena flow is verified");
        assert!(boss_def_portable(4), "definition 4 arena flow is verified");
        assert!(boss_def_portable(5), "definition 5 arena flow is verified");
        let mut c = cam_at(80, 448);
        c.scene_state = 0x0F;
        c.finish_transition();
        assert_eq!(
            c.begin_scene_load(0x0D),
            Some((32, 32)),
            "(sMusicRequest, sLevelSong)"
        );
        assert_eq!(c.scene_state, 0xF0, "the park");
        assert!(c.boss_mode(), "$0652 now picks the boss driver");
        assert!(!c.scene_load_pending, "consumed");
        // Level $18 keeps its fight-phase song — but only for sMusicRequest;
        // `sLevelSong` stays 32 ($0ECA wrote both before the $0ED9 arm).
        let mut c = cam_at(80, 448);
        c.scene_state = 0x01;
        c.boss_fight_phase = 2;
        c.finish_transition();
        assert_eq!(c.begin_scene_load(0x18), Some((35, 32)));
    }

    /// `$0EB0`'s scene branch is reported, never taken: taking it would park
    /// `step` (via `scene_state & $F0`) and swap the object pass for the boss
    /// driver while the handlers that clear `$F0` — the bank-15 `$3C000` ones —
    /// are still unported. A play-test regression caught exactly that.
    #[test]
    fn the_scene_branch_is_reported_and_does_not_park_the_camera() {
        let mut c = cam_at(80, 448);
        c.scene_state = 0x0F; // a pipe/scene arm
        c.finish_transition();
        assert!(c.scene_load_pending, "$0EB0's branch was reached");
        assert!(c.take_blocked_scene_load(), "and it is reported");
        assert_eq!(c.scene_state, 0x0F, "but the $F0 park is NOT taken");
        assert!(!c.boss_mode(), "so the object pass stays the level engine");
        assert!(!c.scene_load_pending, "consumed once");
        assert!(!c.take_blocked_scene_load(), "and not twice");
        assert_eq!(c.sfx0, 5, "$0EB0's own tail still runs");
        // The camera still steps: a parked one returns immediately.
        let mut sim = PlayerSim::default();
        c.step(&mut sim);
        assert!(c.scene_state == 0x0F, "still not parked after a step");
    }

    /// `$0EB0`'s two arms are separate `if`s, not if/else: the `sBossDefeated`
    /// clear must run whether or not the scene branch was taken.
    #[test]
    fn the_boss_defeated_clear_is_not_an_else_of_the_scene_branch() {
        // Scene branch taken.
        let mut c = cam_at(80, 448);
        c.scene_state = 0x03;
        c.boss_defeated = 0x25;
        c.finish_transition();
        assert!(c.scene_load_pending);
        assert_eq!(c.boss_defeated, 0x05, "cleared anyway");
        // No scene: the arm must still run.
        let mut c = cam_at(80, 448);
        c.scene_state = 0x00;
        c.boss_defeated = 0x25;
        c.finish_transition();
        assert!(!c.scene_load_pending, "no scene load");
        assert_eq!(c.boss_defeated, 0x05, "and the clear still runs");
    }

    /// The play-test's pipe, against level `$0D`'s real data: its scene tile
    /// (`$10`) sits at X-high-nibble 14, Y-high-nibble 0, and that cell's edge
    /// byte is `$0D` — bit 0, the right-edge arm. So the entry *is* available:
    /// with the camera in that cell and Mario far enough right of it, `$0B5E`
    /// must set the transition.
    #[test]
    fn level_13_pipe_cell_fires_the_right_edge_arm() {
        let ed = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../extracted/levels/level14.edge.bin"
        ))
        .unwrap();
        let sc = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../extracted/levels/level14.scene.bin"
        ))
        .unwrap();
        assert_eq!(sc[14], 0x10, "the pipe's scene tile, at (Yhi 0, Xhi 14)");
        assert_eq!(ed[14] & 1, 1, "and that cell carries the right-edge bit");
        let mut c = CameraSim::new(0xEB0, 0x0000);
        c.edge_map.copy_from_slice(&ed[..48]);
        c.scene_map = sc.to_vec();
        assert_eq!(c.edge_byte(), 0x0D, "read at the camera's own cell");
        // Mario's screen X is (marioX - camX) + 96; the arm needs >= $A1.
        let mut sim = PlayerSim::default();
        sim.x = 0xEB0 + 0x41;
        c.step(&mut sim);
        assert_eq!(c.transition, 1, "$0B5E starts the right transition");
        assert_eq!(c.scene_id, 0x10, "and $0F0B latches the pipe's scene");
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
    /// Right, bit5 Left, bit6 Up, bit7 Down.) Select and Start are part of the
    /// test now that `PlayerKeys` carries them: holding either cancels a pan.
    #[test]
    fn crouch_pan_requires_exactly_b_plus_the_direction() {
        let not_b_down: [(&str, PlayerKeys); 8] = [
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
            (
                "B+Down+Start ($8A) — cp $82 tests the whole byte",
                PlayerKeys {
                    down: true,
                    b: true,
                    start: true,
                    ..Default::default()
                },
            ),
            (
                "B+Down+Select ($86)",
                PlayerKeys {
                    down: true,
                    b: true,
                    select: true,
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

    // ---- $0DAF: the autoscroll path -------------------------------------

    /// An autoscroll camera with an all-clear edge table and Mario latched
    /// level with its row, so the vertical follow has nothing to do and every
    /// assertion is about the horizontal path. `sim.phase = 0` is `$FF97`
    /// bit 0 clear, i.e. an even frame — the frames that scroll.
    fn auto_cam(x: u16, y: u16) -> (CameraSim, PlayerSim) {
        let mut c = CameraSim::new(x, y);
        c.edge_map = [0u8; 48];
        c.auto_scroll = 1;
        c.prev_mario_y_lo = y as u8;
        let mut sim = PlayerSim::new();
        sim.x = x;
        sim.y = y as u8 as u16;
        sim.phase = 0;
        (c, sim)
    }

    /// `$0DCA`: `ldh a,[$FF97] / and $01 / ret nz` — the scroll is one pixel
    /// per TWO frames, so an odd frame does nothing at all. An even frame
    /// clears `sScrollLeftEnabled` and sets `sMapScrollFlags` bit 4.
    #[test]
    fn autoscroll_scrolls_one_pixel_on_even_frames_only() {
        let (mut c, mut sim) = auto_cam(0x100, 0x140);
        c.left_enabled = true;
        c.step(&mut sim);
        assert_eq!(c.x, 0x101, "even frame: one px");
        assert!(!c.left_enabled, "$0DCA clears sScrollLeftEnabled");
        assert_eq!(c.map_scroll_flags & 0x10, 0x10, "and sets bit 4");

        let (mut odd, mut sim2) = auto_cam(0x100, 0x140);
        sim2.phase = 1; // $FF97 bit 0 set
        odd.step(&mut sim2);
        assert_eq!(odd.x, 0x100, "odd frame: the routine returns at `ret nz`");
        assert_eq!(odd.map_scroll_flags & 0x10, 0, "and sets no scroll flag");
    }

    /// `$0DAF` never applies the player-speed latches (`$14BF`'s deltas),
    /// never reads the draw's enable flags, and has no ±1 park correction: a
    /// running Mario scrolls the camera exactly one pixel, not `x_sub`.
    #[test]
    fn autoscroll_ignores_the_latches_and_the_enables() {
        let (mut c, mut sim) = auto_cam(0x100, 0x140);
        sim.x_sub = 40; // the normal path would scroll 40 px
        c.right_enabled = false;
        c.left_enabled = false;
        c.step(&mut sim);
        assert_eq!(
            c.x, 0x101,
            "$0DAF: one px, whatever d_r and the enables say"
        );
    }

    /// The carry lands in the camera's high byte, masked to four bits.
    #[test]
    fn autoscroll_wraps_the_camera_high_byte() {
        let (mut c, mut sim) = auto_cam(0x0FFF, 0x140);
        sim.x = 0x00FF;
        c.step(&mut sim);
        assert_eq!(c.x, 0x0000, "$0FFF + 1 masked to $0F | $00");
    }

    /// The right-edge trigger, $0B5E-shaped but with `>= $B0` instead of
    /// `== $B0`, and an outright return below `sMarioScreenX = $A1` — where
    /// `$0B5E`'s tail would scroll instead.
    #[test]
    fn autoscroll_right_edge_needs_b0_and_a1() {
        // Camera at the $B0 column, Mario's screen X $A5: the transition fires.
        let (mut c, mut sim) = auto_cam(0x00B0, 0x140);
        c.edge_map[16] = 0x01; // row 1, cell 0: bit0 (the right edge)
        sim.x = 0x00F5; // sx = $F5 - $B0 + $60 = $A5
        c.step(&mut sim);
        assert_eq!(
            c.transition, 1,
            "sScreenTransition = 1, then the $0F0B latch"
        );
        assert_eq!(c.x, 0x00B0, "the transition arm does not scroll");

        // Same column, screen X $70: below $A1 the routine returns outright —
        // even though the frame is even and the scroll would otherwise run.
        let (mut low, mut sim2) = auto_cam(0x00B0, 0x140);
        low.edge_map[16] = 0x01;
        sim2.x = 0x00C0; // sx = $70
        low.step(&mut sim2);
        assert_eq!(low.transition, 0);
        assert_eq!(
            low.x, 0x00B0,
            "`cp $A1 / ret c`: neither a transition nor a scroll"
        );

        // One pixel below the column: the `>= $B0` test fails, so it scrolls.
        let (mut under, mut sim3) = auto_cam(0x00AF, 0x140);
        under.edge_map[16] = 0x01;
        sim3.x = 0x00F5;
        under.step(&mut sim3);
        assert_eq!(under.transition, 0, "`>= $B0` is not $0B5E's `== $B0`");
        assert_eq!(under.x, 0x00B0, "so the autoscroll px is what happens");
    }

    /// `$0DAF` re-branches straight to `$0C7D`, so the left chain never runs:
    /// the `$50`-column left transition (which warps Mario to x=0, hi+1) cannot
    /// fire on an autoscroll frame.
    #[test]
    fn autoscroll_skips_the_left_chain_entirely() {
        let (mut c, mut sim) = auto_cam(0x0050, 0x140);
        c.edge_map[16] = 0x02; // row 1, cell 0: bit1 (the left edge)
        sim.x = 0x00F5; // sx = $05 < $0F: the left transition's own gate
        c.step(&mut sim);
        assert_eq!(c.transition, 0, "no left transition on an autoscroll frame");
        assert_eq!(sim.x, 0x00F5, "and Mario is not warped to x=0, hi+1");
        assert_eq!(c.x, 0x0051, "the autoscroll px happens instead");
    }

    /// The vertical follow is *shared*: `$0DAF` jumps to `$0C7D`, so an
    /// autoscroll frame still follows Mario down.
    #[test]
    fn autoscroll_still_runs_the_vertical_follow() {
        let (mut c, mut sim) = auto_cam(0x100, 0x140);
        sim.y = 0x40;
        c.prev_mario_y_lo = 0x30; // descended 16 px
        c.step(&mut sim);
        assert!(c.y > 0x140, "the $0C7D follow ran (y {} )", c.y);
    }

    // ---- $0DED: the horizontal transitions (tr=1 / tr=2) ----------------

    /// A right-edge transition. `$0B5E`'s bit0 arm fires at the `$B0` column
    /// once Mario's screen X reaches `$A1`; the scroll then moves the camera
    /// 4 px and Mario 1 px per frame until the camera reaches the `$50` column
    /// (one full 160-px screen, wrapping the low byte), and the `$0EB0` tail
    /// ends it with `sfx0 = 5`.
    ///
    /// No capture has ever run this: level 1's `$4A` exit bell fires at x 4056
    /// and the col-255 wall blocks x >= 4068, so both horizontal transitions are
    /// unreachable there. The port has carried this path since the transition
    /// work, exercised only by `screen_transition1`'s *vertical* pair.
    #[test]
    fn a_right_edge_transition_scrolls_to_the_50_column() {
        // Camera on the $B0 column, row 1 (cell index 16); Mario's screen X
        // $A5 ($F5 - $B0 + $60).
        let mut c = cam_at(0x00B0, 0x0140);
        c.edge_map[16] = 0x01; // bit0: the right edge
        let mut sim = PlayerSim::new();
        sim.x = 0x00F5;
        sim.y = 0x0140;
        c.step(&mut sim);
        assert_eq!(c.transition, 1, "$0B5E's bit0 arm fires");
        let (mario_at_arm, cam_at_arm) = (sim.x, c.x);

        let mut frames = 0u16;
        while c.transition != 0 && frames < 64 {
            c.step(&mut sim);
            frames += 1;
        }
        assert_eq!(c.transition, 0, "the scroll finishes");
        assert_eq!(c.x as u8, 0x50, "at the $50 column");
        assert_eq!(
            c.x,
            (cam_at_arm + 4 * frames) & 0x0FFF,
            "4 px of camera per frame, {} of them",
            frames
        );
        assert_eq!(sim.x, mario_at_arm + frames, "and 1 px of Mario");
        assert_eq!(c.sfx0, 5, "$0EB0's tail");
        assert_eq!(c.map_scroll_flags, 0x10, "$0DED's right-scroll flag");
    }

    /// The left transition is the mirror image *plus a warp*: `$0BE5` fires at
    /// the `$50` column once Mario's screen X drops below `$0F`, and it rewrites
    /// Mario x to 0 with the camera's high byte + 1 (`$FFC2/$FFC3`) before the
    /// scroll walks the camera back to the `$B0` column at 4 px/frame.
    #[test]
    fn a_left_edge_transition_warps_mario_then_scrolls_to_the_b0_column() {
        let mut c = cam_at(0x0050, 0x0140);
        c.edge_map[16] = 0x02; // bit1: the left edge
        let mut sim = PlayerSim::new();
        sim.x = 0x02F5; // screen X $05, and Xhi 2 for the warp
        sim.y = 0x0140;
        c.step(&mut sim);
        assert_eq!(c.transition, 2, "$0BE5's bit1 arm fires");
        assert_eq!(sim.x, 0x0300, "Mario warps to x=0 with the camera's hi+1");
        let mario_at_warp = sim.x;
        let cam_at_warp = c.x;

        let mut frames = 0u16;
        while c.transition != 0 && frames < 64 {
            c.step(&mut sim);
            frames += 1;
        }
        assert_eq!(c.transition, 0, "the scroll finishes");
        assert_eq!(c.x as u8, 0xB0, "at the $B0 column");
        assert_eq!(c.x, cam_at_warp.wrapping_sub(4 * frames) & 0x0FFF);
        assert_eq!(sim.x, mario_at_warp.wrapping_sub(frames as u16));
        assert_eq!(c.sfx0, 5, "$0EB0's tail");
        assert_eq!(c.map_scroll_flags, 0x20, "$0DED's left-scroll flag");
    }

    /// The two arms need their own edge bit: with bit1 (and not bit0) at the
    /// `$B0` column — the column bit0 is the right arm's — nothing fires, and
    /// the frame falls through to the normal scroll.
    #[test]
    fn the_transition_arms_need_their_own_edge_bit() {
        let mut c = cam_at(0x00B0, 0x0140);
        c.edge_map[16] = 0x02; // the left edge's bit at the right arm's column
        let mut sim = PlayerSim::new();
        sim.x = 0x00F5; // screen X $A5, past the right arm's $A1 gate
        sim.y = 0x0140;
        c.step(&mut sim);
        assert_eq!(c.transition, 0, "no transition without bit0");
        assert_eq!(c.x, 0x00B0, "and no scroll either: Mario is at the $B0 arm");
    }

    // ---- $0F0B: the scene latch, and the map that feeds it --------------

    /// Level 3's real `sSceneMap` (`extracted/levels/level03.scene.bin`): id 7
    /// at cell (row 0, Xhi 4) and id 6 at (row 1, Xhi 5) — the two scenes of
    /// the first scene-system slice. The index is
    /// `(Yhi & $0F) * 16 + (Xhi & $0F)`, so those are entries 4 and 21.
    fn level3_scene_map() -> Vec<u8> {
        let mut m = vec![0u8; 48];
        m[4] = 7;
        m[21] = 6;
        m
    }

    /// `$0F0B` latches `sSceneId` from the cell under Mario, and does its two
    /// side writes: `sMapEventFlag = 255` and `sPaletteFadeStep = 0`. Until now
    /// nothing filled `scene_map`, so this always latched 0.
    #[test]
    fn scene_lookup_latches_the_id_from_the_cell_under_mario() {
        let mut c = CameraSim::new(0, 0x140);
        c.scene_map = level3_scene_map();
        let mut sim = PlayerSim::new();
        sim.x = 0x0400; // Xhi 4
        sim.y = 0x0030; // Yhi 0 -> index 4 -> id 7
        c.scene_lookup(&sim);
        assert_eq!(c.scene_id, 7);
        assert_eq!(c.map_event_flag, 0xFF, "$0F0B sets sMapEventFlag = 255");
        assert_eq!(c.palette_fade_step, 0, "and clears sPaletteFadeStep");

        sim.x = 0x0500; // Xhi 5
        sim.y = 0x0100; // Yhi 1 -> index 21 -> id 6
        c.scene_lookup(&sim);
        assert_eq!(c.scene_id, 6);
    }

    /// The cell nibble is masked (`$FFC3 & $0F`), so a high byte above $0F
    /// wraps within its row; and a cell past the 48-byte map latches 0 instead
    /// of reading out of range — which is also what an absent map does.
    #[test]
    fn scene_lookup_masks_the_cell_and_tolerates_a_short_map() {
        let mut c = CameraSim::new(0, 0x140);
        c.scene_map = level3_scene_map();
        let mut sim = PlayerSim::new();
        sim.x = 0x1400; // Xhi $14 -> & $0F = 4: the same cell as above
        sim.y = 0x0030;
        c.scene_lookup(&sim);
        assert_eq!(c.scene_id, 7, "$FFC3 is masked with $0F");

        let mut none = CameraSim::new(0, 0x140);
        none.scene_id = 0xEE;
        sim.y = 0x0300; // Yhi 3 -> index 52, past the 3-row map
        none.scene_lookup(&sim);
        assert_eq!(none.scene_id, 0, "outside the map there is no scene");
    }

    // ---- the up-lock warp and the `$0D97` previous-Y latch --------------

    /// The up-lock arm warps Mario (`$FFC0 = 0`, `home.asm:1786`) and then falls
    /// into the `$0D97` tail, which latches **the live** `$FFC0` into
    /// `sPrevMarioY` (`home.asm:1860-1861`). So the frame the warp happens on is
    /// also the frame the latch picks up the warped value — the port used to
    /// keep the value the body entered with, leaving the next frame's `dv` short
    /// by the pre-warp low byte.
    #[test]
    fn the_up_lock_warp_latches_the_warped_y_not_the_entry_value() {
        // Camera on the $48 lock row; Mario high up the screen (his Y low byte
        // $C0 against a latch of 0 is `dv = $C0`, the "moved up" branch) and
        // `sy98 = 0 - $48 + $62 = $1A < $1B`, the warp's own gate.
        let mut c = cam_at(0x0100, 0x0048);
        c.edge_map[1] = 0x04; // camera row 0, Xhi 1: bit2 (the up lock)
        c.prev_mario_y_lo = 0x00;
        let mut sim = PlayerSim::new();
        sim.y = 0x00C0;
        c.step(&mut sim);

        assert_eq!(c.transition, 4, "the up transition arms");
        assert_eq!(sim.y, 0x0000, "and Mario warps to Y=0 with the camera's hi");
        assert_eq!(
            c.prev_mario_y_lo, 0x00,
            "$0D97 latches the live $FFC0, i.e. the warped 0 — not the $C0 the body entered with"
        );
    }

    // ---- $2062 scroll computation and $54E9 sCameraPanTimer --------------

    #[test]
    fn scroll_update_matches_2062_rom_math() {
        let mut c = CameraSim::new(0x0180, 0x0140);
        // Initially pan_timer is 0; scroll_x = $180 - 80 = $130 = $30; scroll_y = $140 - 72 = $F8
        assert_eq!(c.scroll_x, (0x0180u16.wrapping_sub(80)) as u8);
        assert_eq!(c.scroll_y, (0x0140u16.wrapping_sub(72)) as u8);

        // With pan_timer set to 52 ($34): low 2 bits are 0 -> subtracts 0
        c.pan_timer = 52;
        c.update_scroll();
        assert_eq!(c.scroll_y, (0x0140u16.wrapping_sub(72)) as u8);

        // With pan_timer set to 55 ($37): low 2 bits are 3 -> subtracts 3
        c.pan_timer = 55;
        c.update_scroll();
        let expected = 0x0140u16.wrapping_sub(72).wrapping_sub(3) as u8;
        assert_eq!(c.scroll_y, expected);
    }

    #[test]
    fn pan_timer_decrements_once_per_tick_and_stops_at_zero() {
        let mut c = CameraSim::new(0, 0);
        c.pan_timer = 2;
        c.tick_pan_timer();
        assert_eq!(c.pan_timer, 1);
        c.tick_pan_timer();
        assert_eq!(c.pan_timer, 0);
        c.tick_pan_timer();
        assert_eq!(c.pan_timer, 0, "does not underflow below zero");
    }

    #[test]
    fn pan_scroll_latches_before_tick_modeling_two_frame_timing() {
        let mut c = CameraSim::new(0x0180, 0x0140);
        c.pan_timer = 3;
        c.update_scroll();
        let latched_scroll = c.scroll_y;
        assert_eq!(
            latched_scroll,
            0x0140u16.wrapping_sub(72).wrapping_sub(3) as u8
        );

        // $54E9 ticks pan_timer from 3 -> 2; latched scroll_y remains unchanged until update_scroll
        c.tick_pan_timer();
        assert_eq!(c.pan_timer, 2);
        assert_eq!(
            c.scroll_y, latched_scroll,
            "$2F72 reads previous frame latched scroll"
        );

        // Next frame's $2062 update re-latches scroll with pan_timer 2
        c.update_scroll();
        assert_eq!(c.scroll_y, 0x0140u16.wrapping_sub(72).wrapping_sub(2) as u8);
    }

    #[test]
    fn pan_draw_preserves_pre_decrement_scroll_and_uses_post_decrement_oam_phase() {
        let mut c = CameraSim::new(0x180, 0x140);
        c.pan_timer = 55;
        c.update_player_pan(0x140, false, true);
        assert_eq!(c.pan_timer, 54, "$54E9 counter byte");
        assert_eq!((c.scroll_pan, c.player_pan), (3, 2));
        assert_eq!(c.scroll_y, 0xF5, "$2062 latched before the decrement");
        assert_eq!(c.scroll_x, 0x30);
        // The next player update's $2F72 still reads the previous scroll.
        assert_eq!(c.scroll_y, 0xF5);
        c.update_player_pan(0x140, false, true);
        assert_eq!((c.pan_timer, c.scroll_pan, c.player_pan), (53, 2, 1));
        assert_eq!(c.scroll_y, 0xF6);
    }

    #[test]
    fn pan_draw_handles_both_rom_seed_values_and_zero_without_underflow() {
        for (seed, counter, scroll, player) in
            [(52, 51, 0, 3), (56, 55, 0, 3), (1, 0, 1, 0), (0, 0, 0, 0)]
        {
            let mut c = CameraSim::new(80, 72);
            c.pan_timer = seed;
            c.update_player_pan(72, false, true);
            assert_eq!(
                (c.pan_timer, c.scroll_pan, c.player_pan),
                (counter, scroll, player)
            );
            assert_eq!(c.scroll_y, 0u8.wrapping_sub(scroll));
        }
    }

    #[test]
    fn pan_draw_latches_when_the_boss_camera_is_parked_and_freezes_outside_level() {
        let mut c = CameraSim::new(80, 72);
        c.scene_state = 0xF0;
        c.pan_timer = 3;
        c.update_player_pan(72, false, true);
        assert_eq!((c.pan_timer, c.scroll_y, c.player_pan), (2, 253, 2));
        c.update_player_pan(72, false, false); // paused/death fade/exit fade
        assert_eq!((c.pan_timer, c.scroll_y, c.player_pan), (2, 253, 2));
    }

    #[test]
    fn pan_dead_draw_keeps_scroll_latched_and_skips_the_hidden_screen_band() {
        let mut c = CameraSim::new(80, 72);
        c.pan_timer = 3;
        c.update_scroll();
        c.pan_timer = 2;
        c.update_player_pan(72, true, true);
        assert_eq!((c.pan_timer, c.scroll_y, c.player_pan), (1, 253, 1));
        for screen_y in [0xE0u8, 0xFF] {
            let y = (screen_y.wrapping_add(72).wrapping_sub(98)) as u16;
            c.pan_timer = 3;
            c.update_player_pan(y, true, true);
            assert_eq!((c.pan_timer, c.scroll_y, c.player_pan), (3, 253, 0));
        }
    }

    #[test]
    fn passive_scroll_deltas_move_camera_and_clear() {
        let mut c = CameraSim::new(80, 72);
        c.right_enabled = true;
        let mut sim = PlayerSim::new();
        sim.x = 80;
        sim.y = 72;
        sim.x_sub = 0;
        c.scroll_delta_right = 2;

        c.step(&mut sim);
        assert_eq!(c.x, 83);
        assert_eq!(c.scroll_delta_right, 0);
        assert_eq!(c.scroll_delta_left, 0);

        // Active x_sub overrides passive scroll delta
        c.right_enabled = true;
        c.scroll_delta_right = 2;
        sim.x_sub = 3;
        c.step(&mut sim);
        assert_eq!(c.x, 87);
        assert_eq!(c.scroll_delta_right, 0);
    }

    #[test]
    fn transition_updates_scroll_coordinates_and_prev_mario_y_each_frame() {
        let mut c = CameraSim::new(0x00B0, 0x0048);
        c.edge_map[0] = 0x01; // bit0: right edge
        let mut sim = PlayerSim::new();
        sim.x = 0x00B0 + 0x41; // sx >= $A1 -> triggers transition 1
        sim.y = 0x0048;

        c.step(&mut sim);
        assert_eq!(c.transition, 1, "right transition started");

        let initial_scroll_x = c.scroll_x;

        // Step one transition frame
        c.step(&mut sim);
        assert_eq!(c.x, 0x00B4, "camera X advances +4 during transition");
        assert_eq!(
            c.scroll_x,
            c.x.wrapping_sub(80) as u8,
            "scroll_x is recomputed during transition frames"
        );
        assert_ne!(
            c.scroll_x, initial_scroll_x,
            "scroll_x updates rather than remaining frozen"
        );
        assert_eq!(
            c.prev_mario_y_lo, sim.y as u8,
            "prev_mario_y_lo tracks Mario Y during transition"
        );

        // Run transition to completion
        while c.transition != 0 {
            c.step(&mut sim);
        }
        assert_eq!(c.transition, 0, "transition completed");

        // The first frame after transition completes should not experience a sudden camera jump
        let cam_y_at_end = c.y;
        c.step(&mut sim);
        assert_eq!(
            c.y, cam_y_at_end,
            "camera Y does not snap because prev_mario_y_lo was maintained"
        );
    }
}
