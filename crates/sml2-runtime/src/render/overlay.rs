//! Debug overlay geometry in view pixels: `$2EF5` contact boxes and the
//! native 160×144 screen. Read-only; the desktop paints these over the frame.

use super::view;
use crate::session::{Session, SessionState};
use sml2_core::objects::{ContactBox, SLOT_COUNT};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayKind {
    Mario,
    Object,
    /// A slot `ObjectSim::contact_excluded` keeps out of every contact.
    Excluded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlayBox {
    pub kind: OverlayKind,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

fn to_view(kind: OverlayKind, b: ContactBox, (mx, my): (i32, i32)) -> OverlayBox {
    OverlayBox {
        kind,
        x: b.x0 - 8 + mx,
        y: b.y0 - 16 + my,
        w: b.x1 - b.x0,
        h: b.y1 - b.y0,
    }
}

impl Session {
    /// Mario's and every contact-gated slot's `$2EF5` box, in the camera
    /// the last object step used. Levels and demos only; no Mario box while
    /// `step` skips `collide` (travelling) or `collide` returns early (dead).
    pub fn overlay_boxes(&self, view: (usize, usize)) -> Vec<OverlayBox> {
        let Some(ctx) = self.level_ctx() else {
            return Vec::new();
        };
        let (mx, my) = view::level_margin(view);
        let margin = (mx, my + self.hud_style.lift());
        let obj = &ctx.obj;
        let (cx, cy) = (obj.camera_position.x, obj.camera_position.y);
        let mut out = Vec::new();
        let contact = obj.mario_dead == 0 && !obj.mario.travelling;
        if contact && ctx.spr.as_ref().is_some_and(|s| s.player.is_some()) {
            out.push(to_view(
                OverlayKind::Mario,
                obj.mario_contact_box(cx, cy),
                margin,
            ));
        }
        for i in 0..SLOT_COUNT {
            if let Some(b) = obj.slot_contact_box(i, cx, cy) {
                let kind = if obj.contact_excluded(i) {
                    OverlayKind::Excluded
                } else {
                    OverlayKind::Object
                };
                out.push(to_view(kind, b, margin));
            }
        }
        out
    }

    /// The native screen's `(x, y, w, h)` inside an enlarged level, demo or
    /// overworld view.
    pub fn native_border(&self, view: (usize, usize)) -> Option<(i32, i32, i32, i32)> {
        let shown = self.level_ctx().is_some() || matches!(self.state, SessionState::Overworld(_));
        if !shown || !view::enlarged(view) {
            return None;
        }
        let (mx, my) = if self.level_ctx().is_some() {
            view::level_margin(view)
        } else {
            view::margin(view)
        };
        Some((mx, my, 160, 144))
    }
}

#[cfg(test)]
mod tests {
    use super::OverlayKind;
    use crate::content::SessionAssets;
    use crate::session::{FrameInput, Session, SessionState};

    #[test]
    fn no_boxes_outside_levels_and_demos() {
        let session = Session::new(SessionAssets::load().unwrap());
        assert!(session.overlay_boxes((384, 216)).is_empty());
    }

    fn mario_box(session: &Session, view: (usize, usize)) -> (i32, i32, i32, i32) {
        let b = session
            .overlay_boxes(view)
            .into_iter()
            .find(|b| b.kind == OverlayKind::Mario)
            .expect("Mario's box");
        (b.x, b.y, b.w, b.h)
    }

    #[test]
    fn the_mario_box_sits_on_mario_and_shifts_by_the_view_margin() {
        let mut session = Session::new(SessionAssets::load().unwrap());
        session.begin_level(0, 0).unwrap();
        for _ in 0..8 {
            session.advance(FrameInput::default());
        }
        let SessionState::InLevel(ctx) = &session.state else {
            panic!("still in the level")
        };
        let spr = ctx.spr.as_ref().unwrap();
        let (px, py, _) = spr.player.unwrap();
        let c = ctx.obj.camera_position;
        assert_eq!(
            (px, py),
            (ctx.obj.mario.x, ctx.obj.mario.y),
            "standing still"
        );
        assert_eq!(
            (c.x, c.y),
            (ctx.cam.x, ctx.cam.y),
            "camera unchanged since the step"
        );
        // The renderer's native player base: OAM (x − cam + 96, y − cam + 98)
        // drawn at (−8, −16 − player_pan).
        let drawn = (
            i32::from(px) - i32::from(c.x) + 96 - 8,
            i32::from(py) - i32::from(c.y) + 98 - 16 - i32::from(spr.player_pan),
        );
        let e = if ctx.obj.mario.powerup != 0 { 8 } else { 0 };
        let (x, y, w, h) = mario_box(&session, (160, 144));
        assert_eq!(
            (x, y, w, h),
            (
                drawn.0 - 4,
                drawn.1 + i32::from(spr.player_pan) - e,
                10,
                21 + e
            )
        );
        assert!(x >= 0 && y >= 0 && x + w <= 160 && y + h <= 144);
        let (wx, wy, ww, wh) = mario_box(&session, (384, 216));
        assert_eq!((wx - x, wy - y, ww, wh), (112, 54, w, h));
    }

    #[test]
    fn no_mario_box_while_no_contact_is_possible() {
        let mut session = Session::new(SessionAssets::load().unwrap());
        session.begin_level(0, 0).unwrap();
        session.advance(FrameInput::default());
        let has_mario = |s: &Session| {
            s.overlay_boxes((160, 144))
                .iter()
                .any(|b| b.kind == OverlayKind::Mario)
        };
        assert!(has_mario(&session));
        let SessionState::InLevel(ctx) = &mut session.state else {
            panic!()
        };
        ctx.obj.mario.travelling = true;
        assert!(!has_mario(&session), "travelling");
        let SessionState::InLevel(ctx) = &mut session.state else {
            panic!()
        };
        ctx.obj.mario.travelling = false;
        ctx.obj.mario_dead = 15;
        assert!(!has_mario(&session), "dead");
    }

    #[test]
    fn the_native_border_shows_only_in_enlarged_views() {
        let mut session = Session::new(SessionAssets::load().unwrap());
        assert!(matches!(session.state, SessionState::Overworld(_)));
        assert_eq!(session.native_border((160, 144)), None);
        assert_eq!(session.native_border((384, 216)), Some((112, 36, 160, 144)));
        session.begin_level(0, 0).unwrap();
        assert_eq!(session.native_border((160, 144)), None);
        assert_eq!(session.native_border((384, 216)), Some((112, 54, 160, 144)));
    }
}
