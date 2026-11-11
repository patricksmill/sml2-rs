//! Shared-screen camera for local co-op.
//!
//! The ROM camera (`$0B2A`) leads a single Mario: it scrolls by his speed
//! latches and parks him at a fixed screen column. Two players need a camera
//! that follows the party instead. This follows New Super Mario Bros. Wii's
//! rules: the camera tracks only players still in play (`daPyMng_c::
//! getActScrollInfo` skips bubbled players), and the screen edges hold the
//! players (`daPlBase_c::checkDispSideLemit`, applied by the frame loop).
//! NSMBW's own camera arithmetic (`dBgGm_c::execute`) has no decompiled
//! source, so the follow itself is this module's design: centre the live
//! players' horizontal span, keep their vertical midpoint inside the ROM's
//! follow band, move at most [`MAX_STEP`] pixels a frame, and stop at the
//! same `sScreenEdgeMap` locks the ROM camera stops at.
//!
//! Screen transitions keep the ROM's shape: at a lock column/row the
//! player pushing past it starts `sScreenTransition`, and `$0DED` slides the
//! camera while walking that player (the *leader*) into the next room.

use super::CameraSim;
use crate::player_sim::PlayerSim;

/// Pixels the party's centre may drift from the screen centre before the
/// camera follows, so small steps and turns do not move the screen.
pub const DEAD_ZONE: i32 = 8;
/// Fastest follow, in pixels per frame: the transition scroll's speed, above
/// any running or falling speed.
pub const MAX_STEP: i32 = 4;
/// Screen X (`x - cam + 96`) the party's horizontal centre settles on: the
/// middle of the `$10..=$A0` span `$0FBF` allows on the native screen.
pub const CENTRE_X: i32 = 0x58;
/// Screen Y band (`y - cam + 96`) the vertical midpoint is held in: the ROM
/// follow's own `$28` up and `$50` down thresholds.
pub const BAND_Y: (i32, i32) = (0x28, 0x50);

/// One player's part in this frame's shared camera.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CoopPlayer {
    /// In play: not bubbled, not dying, not carried by another's transition.
    pub live: bool,
    /// Held against the left / right screen edge while walking into it.
    pub pressing_left: bool,
    pub pressing_right: bool,
}

/// What the shared camera did this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CoopCameraStep {
    /// The player whose push started a screen transition this frame.
    pub started_by: Option<usize>,
    /// A transition ended this frame.
    pub finished: bool,
}

fn signed(v: u16) -> i32 {
    i32::from(v as i16)
}

/// Move `from` toward keeping `pos` inside `[lo, hi]` (screen-relative
/// distance `pos - from`), at most [`MAX_STEP`] a frame.
fn follow(rel: i32, lo: i32, hi: i32) -> i32 {
    if rel < lo {
        (rel - lo).max(-MAX_STEP)
    } else if rel > hi {
        (rel - hi).min(MAX_STEP)
    } else {
        0
    }
}

impl CameraSim {
    /// One frame of the shared camera. `players` and `roles` are in player
    /// order. A player travelling a pipe owns the camera through the ROM
    /// `step`, so pipe rooms and their transitions behave as in one-player
    /// play; the caller handles the other player (NSMBW bubbles players who
    /// do not take the pipe, `dAcPy_c::setBalloonInNextgoto`).
    pub fn step_coop(
        &mut self,
        players: [&mut PlayerSim; 2],
        roles: [CoopPlayer; 2],
    ) -> CoopCameraStep {
        let mut players = players;
        // Conveyor and carrier pushes feed only the ROM's speed-latched
        // scroll, which this camera does not use.
        for p in players.iter_mut() {
            p.conveyor_scroll_right = 0;
            p.conveyor_scroll_left = 0;
        }
        self.scroll_delta_right = 0;
        self.scroll_delta_left = 0;
        let transition_before = self.transition;
        let mut out = CoopCameraStep::default();

        if self.scene_state & 0xF0 != 0 {
            return out;
        }
        if self.boss_defeated != 0 && self.boss_defeated & 0xF0 == 0 {
            return out;
        }
        if self.transition != 0 {
            // `$0DED` walks only the leader; the caller placed the other
            // player beside it when the transition started.
            let leader = self.coop_leader.min(1);
            let (x0, y0) = (players[leader].x, players[leader].y);
            self.run_transition(&mut *players[leader]);
            let (dx, dy) = (
                players[leader].x.wrapping_sub(x0),
                players[leader].y.wrapping_sub(y0),
            );
            let other = 1 - leader;
            if roles[other].live {
                players[other].x = players[other].x.wrapping_add(dx);
                players[other].y = players[other].y.wrapping_add(dy);
                players[other].walk_anim_tick(1);
            }
            self.prev_mario_y_lo = players[leader].y as u8;
            self.update_scroll();
            out.finished = transition_before != 0 && self.transition == 0;
            return out;
        }
        if let Some(owner) = (0..2).find(|&i| roles[i].live && players[i].pipe_dir >= 0x10) {
            self.coop_leader = owner;
            self.step(&mut *players[owner]);
            self.update_enables(players[owner].x);
            if self.transition != 0 {
                out.started_by = Some(owner);
            }
            return out;
        }

        let live = || (0..2usize).filter(move |&i| roles[i].live);
        let Some(lead) = live().next() else {
            self.update_scroll();
            return out;
        };
        let screen_x = |p: &PlayerSim, cam: u16| signed(p.x.wrapping_sub(cam)) + 96;
        let screen_y = |p: &PlayerSim, cam: u16| signed(p.y.wrapping_sub(cam)) + 96;

        // Horizontal.
        if self.auto_scroll != 0 {
            // The scroll path is the level's; the right-most player owns the
            // end-of-course transition check.
            let right = live()
                .max_by_key(|&i| screen_x(players[i], self.x))
                .unwrap_or(lead);
            self.update_enables(players[right].x);
            self.coop_leader = right;
            self.run_autoscroll(&mut *players[right]);
            if self.transition != 0 {
                out.started_by = Some(right);
            }
        } else {
            let (lo, hi) = live().fold((i32::MAX, i32::MIN), |(lo, hi), i| {
                let sx = screen_x(players[i], self.x);
                (lo.min(sx), hi.max(sx))
            });
            let centre = (lo + hi) / 2;
            // A player pushing an edge wants room: centre exactly, so the
            // screen keeps moving until the trailing player meets the far edge.
            let pushing = live().any(|i| roles[i].pressing_left || roles[i].pressing_right);
            let zone = if pushing { 0 } else { DEAD_ZONE };
            let dx = follow(centre, CENTRE_X - zone, CENTRE_X + zone);
            if dx != 0 {
                self.x = self.world.wrap(self.x.wrapping_add_signed(dx as i16));
                self.map_scroll_flags |= if dx > 0 { 0x10 } else { 0x20 };
            }
            // The ROM locks: `$B0` with bit 0, `$50` with bit 1, read at the
            // cell the camera now stands in.
            let edges = self.edge_bits();
            let hi_byte = self.x & 0xFF00;
            if edges & 1 != 0 && (self.x as u8) > 0xB0 {
                self.x = hi_byte | 0xB0;
            }
            if edges & 2 != 0 && (self.x as u8) < 0x50 {
                self.x = hi_byte | 0x50;
            }
            let edges = self.edge_bits();
            if edges & 1 != 0 && (self.x as u8) == 0xB0 {
                if let Some(i) = live().find(|&i| roles[i].pressing_right) {
                    self.coop_leader = i;
                    self.transition = 1;
                    self.scene_lookup(players[i]);
                    out.started_by = Some(i);
                }
            } else if edges & 2 != 0 && (self.x as u8) == 0x50 {
                if let Some(i) = live().find(|&i| roles[i].pressing_left) {
                    // `$0BE5`: Mario warps to x = 0 of the next high byte.
                    let p = &mut *players[i];
                    p.x = self.world.wrap((p.x >> 8).wrapping_add(1) << 8);
                    self.coop_leader = i;
                    self.transition = 2;
                    self.scene_lookup(players[i]);
                    out.started_by = Some(i);
                }
            }
        }

        // Vertical: hold the midpoint inside the ROM follow band.
        if self.transition == 0 {
            let (lo, hi) = live().fold((i32::MAX, i32::MIN), |(lo, hi), i| {
                let sy = screen_y(players[i], self.y);
                (lo.min(sy), hi.max(sy))
            });
            let dy = follow((lo + hi) / 2, BAND_Y.0, BAND_Y.1);
            if dy != 0 {
                self.y = self.world.wrap(self.y.wrapping_add_signed(dy as i16));
                self.map_scroll_flags |= if dy > 0 { 0x80 } else { 0x40 };
            }
            let edges = self.edge_bits();
            let hi_byte = self.y & 0xFF00;
            if edges & 4 != 0 && (self.y as u8) < 0x48 {
                self.y = hi_byte | 0x48;
            }
            if edges & 8 != 0 && (self.y as u8) > 0xC0 {
                self.y = hi_byte | 0xC0;
            }
            let edges = self.edge_bits();
            let sy98 = |p: &PlayerSim, cam: u16| signed(p.y.wrapping_sub(cam)) + 98;
            if edges & 8 != 0 && (self.y as u8) == 0xC0 {
                // `$0CEC`: falling past `sMarioScreenY $90` at the `$C0` row.
                if let Some(i) =
                    live().find(|&i| players[i].vvel < 0x80 && sy98(players[i], self.y) >= 0x90)
                {
                    self.coop_leader = i;
                    self.transition = 8;
                    self.scene_lookup(players[i]);
                    self.map_scroll_flags |= 0x40;
                    out.started_by = Some(i);
                }
            } else if edges & 4 != 0 && (self.y as u8) == 0x48 {
                // `$0D05`: rising past `$1B` at the `$48` row warps the leader
                // to Y = 0 of the camera's high byte.
                if let Some(i) = live().find(|&i| sy98(players[i], self.y) < 0x1B) {
                    players[i].y = self.world.wrap(self.y & 0xFF00);
                    self.coop_leader = i;
                    self.transition = 4;
                    if self.scene_state & 0xF0 == 0 {
                        self.scene_lookup(players[i]);
                    }
                    out.started_by = Some(i);
                }
            }
        }

        let leader = out.started_by.unwrap_or(self.coop_leader.min(1));
        let leader = if roles[leader].live { leader } else { lead };
        // `$54A7`'s latches stay meaningful for the object engine: the
        // right-most live player's screen X.
        let right = live()
            .max_by_key(|&i| screen_x(players[i], self.x))
            .unwrap_or(lead);
        self.update_enables(players[right].x);
        self.prev_mario_y_lo = players[leader].y as u8;
        self.update_scroll();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(cam: &CameraSim, first_sx: i32, second_sx: i32) -> (PlayerSim, PlayerSim) {
        let at = |sx: i32| {
            let mut p = PlayerSim::new();
            p.x = cam.x.wrapping_add_signed((sx - 96) as i16);
            p.y = cam.y.wrapping_add_signed((0x40 - 96) as i16);
            p.vvel = 0;
            p
        };
        (at(first_sx), at(second_sx))
    }

    const LIVE: CoopPlayer = CoopPlayer {
        live: true,
        pressing_left: false,
        pressing_right: false,
    };

    #[test]
    fn the_camera_centres_the_live_party_at_bounded_speed() {
        let mut cam = CameraSim::new(0x0200, 0x0140);
        let (mut a, mut b) = party(&cam, 0x90, 0xA0);
        let before = cam.x;
        cam.step_coop([&mut a, &mut b], [LIVE; 2]);
        assert_eq!(
            cam.x,
            before + MAX_STEP as u16,
            "far right of centre: full step"
        );

        let mut cam = CameraSim::new(0x0200, 0x0140);
        let (mut a, mut b) = party(&cam, CENTRE_X - 20, CENTRE_X + 20);
        cam.step_coop([&mut a, &mut b], [LIVE; 2]);
        assert_eq!(
            cam.x, 0x0200,
            "a centred party inside the dead zone holds still"
        );
    }

    #[test]
    fn a_bubbled_player_does_not_pull_the_camera() {
        let mut cam = CameraSim::new(0x0200, 0x0140);
        let (mut a, mut b) = party(&cam, CENTRE_X, 0x10);
        let out = CoopPlayer::default();
        cam.step_coop([&mut a, &mut b], [LIVE, out]);
        assert_eq!(cam.x, 0x0200);
    }

    #[test]
    fn the_right_lock_stops_the_camera_and_a_push_starts_the_transition() {
        let mut cam = CameraSim::new(0x02AE, 0x0140);
        cam.edge_map[0x12] = 0x01; // row 1, cell 2: right lock
        let (mut a, mut b) = party(&cam, 0x98, 0xA0);
        cam.step_coop([&mut a, &mut b], [LIVE; 2]);
        assert_eq!(cam.x, 0x02B0, "clamped at the $B0 lock column");
        assert_eq!(cam.transition, 0);

        let pushing = CoopPlayer {
            pressing_right: true,
            ..LIVE
        };
        let step = cam.step_coop([&mut a, &mut b], [LIVE, pushing]);
        assert_eq!(step.started_by, Some(1));
        assert_eq!(cam.transition, 1);
        assert_eq!(cam.coop_leader, 1);
    }

    #[test]
    fn the_transition_walks_the_leader_and_carries_the_other_player() {
        let mut cam = CameraSim::new(0x02B0, 0x0140);
        cam.transition = 1;
        cam.coop_leader = 0;
        let (mut a, mut b) = party(&cam, 0xA0, 0x90);
        let (ax, bx) = (a.x, b.x);
        cam.step_coop([&mut a, &mut b], [LIVE; 2]);
        assert_eq!(cam.x, 0x02B4);
        assert_eq!((a.x, b.x), (ax + 1, bx + 1));
    }

    #[test]
    fn the_vertical_midpoint_stays_in_the_follow_band() {
        let mut cam = CameraSim::new(0x0200, 0x0140);
        let (mut a, mut b) = party(&cam, 0x40, 0x60);
        a.y = cam.y.wrapping_add_signed((0x70 - 96) as i16);
        b.y = a.y;
        cam.step_coop([&mut a, &mut b], [LIVE; 2]);
        assert_eq!(cam.y, 0x0140 + MAX_STEP as u16);
    }
}
