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
//!   `sPrevMarioY`). Moving down scrolls only when Mario's screen Y ≥ `$50`;
//!   moving up only when < `$28` (`$40` when `sMapScrollLimit` is set). No
//!   lead correction. The edge-lock/transition paths (`$C0`/`$48` rows,
//!   flip-screen animations) are omitted — flagged, used only by levels
//!   whose `sScreenEdgeMap` bits are set.
//! - All arithmetic on the low bytes like the engine; the hi byte is masked
//!   to `$0F` (12-bit world).

use crate::player_sim::PlayerSim;

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

    fn edge_bits(&self) -> u8 {
        if let Some(e) = self.edge_now {
            return e;
        }
        let xi = ((self.x >> 8) & 0x0F) as usize;
        let yi = ((self.y >> 8) & 0x0F) as usize;
        let idx = (yi & 0x03) * 16 + xi;
        self.edge_map.get(idx).copied().unwrap_or(0)
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
