//! Motion smoothing: in-level positions between the last two ticks, as
//! screen shifts in output pixels for the layer compositor.

use crate::session::LevelContext;

/// Largest per-tick move (game px, either axis) that is blended; larger moves
/// are warps and draw at the current tick.
pub const SNAP: i32 = 8;
/// Background pixels rendered past each view edge, so a background shifted by
/// up to `SNAP` never exposes unrendered pixels.
pub const BORDER: usize = SNAP as usize;
const MAX_PIXELS: usize = 2048 * 2048;

/// How far through the current tick a frame is drawn (`alpha` in `[0, 1]`) and
/// at how many output pixels per game pixel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Smooth {
    pub alpha: f32,
    pub scale: usize,
}

impl Smooth {
    pub const OFF: Self = Self { alpha: 1.0, scale: 1 };
}

/// The largest divisor of `effective_scale` that keeps a `view` frame within
/// 2048² output pixels, so the GPU still upsamples by a whole number.
pub fn pick_scale(view: (usize, usize), effective_scale: u32) -> usize {
    let n = effective_scale.max(1) as usize;
    (1..=n)
        .rev()
        .find(|s| n % s == 0 && view.0 * s * view.1 * s <= MAX_PIXELS)
        .unwrap_or(1)
}

/// The positions one tick drew with, in background space (world positions
/// against the background's `scroll_pan`-raised camera), so a pan change
/// reads as motion.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MotionSnapshot {
    /// Frames from different levels are never blended.
    pub level: u8,
    pub cam: (u16, u16),
    pub player: Option<(u16, u16)>,
    pub shield: Option<(u16, u16)>,
    pub fireballs: Vec<(u16, u16)>,
    pub bumpers: Vec<(u16, u16)>,
    /// `(slot, position)` per drawn object.
    pub objects: Vec<(u8, (u16, u16))>,
}

impl MotionSnapshot {
    /// `None` where frames are never blended: paused, the scene-reload blank
    /// and boss fights.
    pub fn of(ctx: &LevelContext) -> Option<Self> {
        let s = ctx.spr.as_ref()?;
        if ctx.sim.paused || ctx.scene_reload_blank || ctx.cam.boss_mode() || !s.boss_oam.is_empty() {
            return None;
        }
        let pans = Pans {
            scroll: s.scroll_pan,
            player: s.player_pan,
        };
        let anchored = |&(x, y, _): &(u16, u16, u8)| pans.anchored((x, y));
        let mario = |(x, y, _): (u16, u16, u8)| pans.mario((x, y));
        Some(Self {
            level: ctx.manifest.level_id,
            cam: pans.anchored((ctx.cam.x, ctx.cam.y)),
            player: s.player.map(mario),
            shield: s.player_shield.map(mario),
            fireballs: s.fireballs.iter().map(anchored).collect(),
            bumpers: s.bumpers.iter().map(anchored).collect(),
            objects: s.insts.iter().map(|i| (i.slot, (i.x, i.y))).collect(),
        })
    }
}

/// A tick's pans. The background and objects are drawn `scroll_pan` up; the
/// camera-anchored sprites (`view::sprite_anchor`) are not, and Mario's cells
/// are a further `player_pan` up.
#[derive(Debug, Clone, Copy)]
struct Pans {
    scroll: u8,
    player: u8,
}

impl Pans {
    /// The camera, a fireball or a bumper in background space.
    fn anchored(self, (x, y): (u16, u16)) -> (u16, u16) {
        (x, y.wrapping_sub(u16::from(self.scroll)))
    }

    /// Mario or his shield in background space.
    fn mario(self, at: (u16, u16)) -> (u16, u16) {
        let (x, y) = self.anchored(at);
        (x, y.wrapping_sub(u16::from(self.player)))
    }
}

/// Screen shifts in output pixels. A sprite without its own entry moves with
/// the background (world-fixed against the in-between camera).
#[derive(Debug, Clone, PartialEq)]
pub struct Offsets {
    pub scale: usize,
    pub bg: (i32, i32),
    pub player: (i32, i32),
    pub shield: (i32, i32),
    pub fireballs: Vec<(i32, i32)>,
    pub bumpers: Vec<(i32, i32)>,
    pub objects: Vec<(u8, (i32, i32))>,
}

impl Offsets {
    pub fn none() -> Self {
        Self {
            scale: 1,
            bg: (0, 0),
            player: (0, 0),
            shield: (0, 0),
            fireballs: Vec::new(),
            bumpers: Vec::new(),
            objects: Vec::new(),
        }
    }

    /// Background pixels to render past each view edge: only a shifted
    /// background can expose them, at any scale.
    pub fn border(&self) -> usize {
        if self.bg != (0, 0) { BORDER } else { 0 }
    }

    pub fn between(prev: Option<&MotionSnapshot>, cur: Option<&MotionSnapshot>, smooth: Smooth) -> Self {
        let s = smooth.scale.max(1);
        let mut out = Self { scale: s, ..Self::none() };
        let (Some(prev), Some(cur)) = (prev, cur) else {
            return out;
        };
        if prev.level != cur.level {
            return out;
        }
        let Some(cam) = shift(prev.cam, cur.cam, smooth.alpha, s) else {
            return out;
        };
        out.bg = (-cam.0, -cam.1);
        let rel = |p: Option<(u16, u16)>, c: (u16, u16)| {
            let own = p.and_then(|p| shift(p, c, smooth.alpha, s)).unwrap_or((0, 0));
            (own.0 - cam.0, own.1 - cam.1)
        };
        out.player = cur.player.map_or(out.bg, |c| rel(prev.player, c));
        out.shield = cur.shield.map_or(out.bg, |c| rel(prev.shield, c));
        out.fireballs = cur
            .fireballs
            .iter()
            .enumerate()
            .map(|(i, &c)| rel(prev.fireballs.get(i).copied(), c))
            .collect();
        out.bumpers = cur
            .bumpers
            .iter()
            .enumerate()
            .map(|(i, &c)| rel(prev.bumpers.get(i).copied(), c))
            .collect();
        out.objects = cur
            .objects
            .iter()
            .map(|&(slot, c)| {
                let p = prev.objects.iter().find(|o| o.0 == slot).map(|o| o.1);
                (slot, rel(p, c))
            })
            .collect();
        out
    }

    pub fn fireball(&self, i: usize) -> (i32, i32) {
        self.fireballs.get(i).copied().unwrap_or(self.bg)
    }

    pub fn bumper(&self, i: usize) -> (i32, i32) {
        self.bumpers.get(i).copied().unwrap_or(self.bg)
    }

    pub fn object(&self, slot: u8) -> (i32, i32) {
        self.objects.iter().find(|o| o.0 == slot).map_or(self.bg, |o| o.1)
    }
}

/// `(alpha - 1) * (cur - prev)` in output pixels; `None` past `SNAP`.
fn shift(prev: (u16, u16), cur: (u16, u16), alpha: f32, s: usize) -> Option<(i32, i32)> {
    let d = |p: u16, c: u16| i32::from(c.wrapping_sub(p) as i16);
    let (dx, dy) = (d(prev.0, cur.0), d(prev.1, cur.1));
    if dx.abs() > SNAP || dy.abs() > SNAP {
        return None;
    }
    let f = (alpha.clamp(0.0, 1.0) - 1.0) * s as f32;
    Some(((dx as f32 * f).round() as i32, (dy as f32 * f).round() as i32))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(cam: (u16, u16)) -> MotionSnapshot {
        MotionSnapshot { cam, ..Default::default() }
    }

    #[test]
    fn scale_is_the_largest_divisor_within_the_pixel_budget() {
        assert_eq!(pick_scale((160, 144), 3), 3);
        assert_eq!(pick_scale((160, 144), 7), 7);
        assert_eq!(pick_scale((256, 144), 15), 5);
        assert_eq!(pick_scale((160, 144), 0), 1);
    }

    #[test]
    fn sprites_moving_with_the_camera_stay_put_and_static_ones_follow_the_background() {
        let mut prev = snap((100, 50));
        prev.player = Some((110, 60));
        prev.objects = vec![(3, (130, 60))];
        let mut cur = snap((104, 50));
        cur.player = Some((114, 60));
        cur.objects = vec![(3, (130, 60)), (5, (140, 60))];
        let o = Offsets::between(Some(&prev), Some(&cur), Smooth { alpha: 0.5, scale: 2 });
        assert_eq!(o.bg, (4, 0));
        assert_eq!(o.player, (0, 0));
        assert_eq!(o.object(3), (4, 0));
        assert_eq!(o.object(5), (4, 0), "a new slot is world-fixed");
        assert_eq!(o.fireball(0), (4, 0));
    }

    #[test]
    fn a_scroll_pan_change_moves_the_background_but_not_the_anchored_sprites() {
        let tick = |scroll| {
            let pans = Pans { scroll, player: 2 };
            MotionSnapshot {
                cam: pans.anchored((100, 50)),
                player: Some(pans.mario((110, 60))),
                shield: Some(pans.mario((110, 60))),
                fireballs: vec![pans.anchored((120, 60))],
                bumpers: vec![pans.anchored((140, 40))],
                objects: vec![(3, (130, 60))],
                ..Default::default()
            }
        };
        let o = Offsets::between(
            Some(&tick(0)),
            Some(&tick(4)),
            Smooth {
                alpha: 0.5,
                scale: 2,
            },
        );
        assert_eq!(o.bg, (0, -4));
        assert_eq!((o.player, o.shield), ((0, 0), (0, 0)));
        assert_eq!((o.fireball(0), o.bumper(0)), ((0, 0), (0, 0)));
        assert_eq!(
            o.object(3),
            o.bg,
            "objects are drawn through the scrolled camera"
        );
    }

    #[test]
    fn warps_pauses_and_full_ticks_draw_the_current_tick() {
        let s = Smooth { alpha: 0.25, scale: 3 };
        let warp = Offsets::between(Some(&snap((100, 50))), Some(&snap((120, 50))), s);
        assert_eq!(warp, Offsets { scale: 3, ..Offsets::none() });
        let missing = Offsets::between(None, Some(&snap((100, 50))), s);
        assert_eq!(missing.bg, (0, 0));
        let done = Offsets::between(
            Some(&snap((100, 50))),
            Some(&snap((104, 50))),
            Smooth { alpha: 1.0, scale: 3 },
        );
        assert_eq!(done.bg, (0, 0));
    }

    #[test]
    fn a_new_level_draws_the_current_tick() {
        let mut prev = snap((100, 50));
        prev.objects = vec![(2, (10, 10))];
        let mut cur = MotionSnapshot {
            level: 1,
            ..snap((102, 50))
        };
        cur.objects = vec![(2, (12, 10))];
        let s = Smooth {
            alpha: 0.5,
            scale: 2,
        };
        let o = Offsets::between(Some(&prev), Some(&cur), s);
        assert_eq!(
            o,
            Offsets {
                scale: 2,
                ..Offsets::none()
            }
        );
    }

    #[test]
    fn a_sprite_that_jumps_is_drawn_world_fixed() {
        let mut prev = snap((100, 50));
        prev.objects = vec![(2, (10, 10))];
        let mut cur = snap((102, 50));
        cur.objects = vec![(2, (200, 10))];
        let o = Offsets::between(Some(&prev), Some(&cur), Smooth { alpha: 0.5, scale: 1 });
        assert_eq!(o.object(2), o.bg);
    }

    #[test]
    fn a_shifted_background_is_bordered_even_at_scale_one() {
        let s = Smooth { alpha: 0.5, scale: 1 };
        let o = Offsets::between(Some(&snap((100, 50))), Some(&snap((102, 50))), s);
        assert_eq!(o.bg, (1, 0));
        assert_eq!(o.border(), BORDER);
        assert_eq!(Offsets::none().border(), 0);
    }

    #[test]
    fn a_level_session_remembers_the_previous_tick() {
        use crate::session::{FrameInput, Session, SessionAssets};
        let mut session = Session::new(SessionAssets::load().unwrap());
        session.begin_level(0, 0).unwrap();
        for _ in 0..2 {
            session.advance(FrameInput::default());
        }
        let before = session.level_ctx().and_then(MotionSnapshot::of);
        assert!(before.is_some());
        session.advance(FrameInput::default());
        assert_eq!(session.motion_prev, before);
        session.enter_title();
        session.advance(FrameInput::default());
        assert!(session.motion_prev.is_none());
    }
}
