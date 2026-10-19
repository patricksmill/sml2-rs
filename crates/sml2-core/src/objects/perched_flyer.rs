//! Native perched flyer enemy domain behavior, ROM `$5A1F6` (anim 52 Mode A & B).
//!
//! Waits perched in place (State 3, with sprite bit 0 set).
//! When Mario approaches horizontally within 24 pixels, it is startled:
//! faces toward Mario, squawks (SFX 23), and flies away upwards and in
//! the opposite direction at 3 px/frame horizontally and 1 px/frame vertically,
//! flapping its wings every 2 frames ($5B521).
//!
//! Follows the idiomatic Rust porting guide: typed actor state, domain methods,
//! zero raw hex offsets or register plumbing.

use crate::actor::{Actor, ContactKind, FrameTick};

pub const TRIGGER_DISTANCE_X: u8 = 24; // ROM $5A215: cp $18
pub const CONTACT_KIND: u8 = 77; // ROM $5A1FB: ld a, 77 (0x4D)
pub const STARTLE_SFX: u8 = 23; // ROM $5A21B: ld a, 23 (sSfxRequest1)

/// Perched flyer movement state ($5A1F6 / $6221 / $6228 / $620C: states 0..=3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerchedFlyerState {
    Init = 0,
    FleeRight = 1,
    FleeLeft = 2,
    Perched = 3,
}

impl PerchedFlyerState {
    pub const fn from_raw(value: u8) -> Self {
        match value {
            1 => Self::FleeRight,
            2 => Self::FleeLeft,
            3 => Self::Perched,
            _ => Self::Init,
        }
    }

    pub const fn to_raw(self) -> u8 {
        self as u8
    }
}

pub struct PerchedFlyerContext {
    pub frame: FrameTick,
    pub mario_ref_x: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerchedFlyerEffect {
    None,
    Initialized,
    Startled { sfx: u8, facing: u8 },
    Flying { state: PerchedFlyerState },
}

/// Domain update for PerchedFlyer (ROM $5A1F6).
pub fn update(actor: &mut Actor, context: PerchedFlyerContext) -> PerchedFlyerEffect {
    let state = PerchedFlyerState::from_raw(actor.state);
    match state {
        PerchedFlyerState::Init => {
            actor.contact_kind = ContactKind::from_raw(CONTACT_KIND);
            actor.state = PerchedFlyerState::Perched.to_raw();
            PerchedFlyerEffect::Initialized
        }
        PerchedFlyerState::Perched => {
            // ROM $5A20C: set bit 0 of sprite
            actor.sprite |= 1;

            // ROM $5B4B4: abs(sMarioRefXLo - sObjectXLo)
            let mario_lo = context.mario_ref_x as u8;
            let actor_lo = actor.position.x as u8;
            let diff = mario_lo.wrapping_sub(actor_lo);
            let dist = if diff & 0x80 == 0 {
                diff
            } else {
                (!diff).wrapping_add(1)
            };

            // ROM $5A215: cp $18 (return if >= 24)
            if dist >= TRIGGER_DISTANCE_X {
                return PerchedFlyerEffect::None;
            }

            // ROM $5B3C6: face toward Mario
            // Note: Mario on right -> state = 2 (FleeLeft), Mario on left -> state = 1 (FleeRight)
            let (next_state, flip, facing) = match context.mario_ref_x.cmp(&actor.position.x) {
                std::cmp::Ordering::Greater => (PerchedFlyerState::FleeLeft.to_raw(), true, 1),
                std::cmp::Ordering::Less => (PerchedFlyerState::FleeRight.to_raw(), false, 0xFF),
                std::cmp::Ordering::Equal => (PerchedFlyerState::FleeRight.to_raw(), false, 0),
            };
            actor.state = next_state;
            actor.sprite_flip.set_horizontal(flip);
            PerchedFlyerEffect::Startled {
                sfx: STARTLE_SFX,
                facing,
            }
        }
        PerchedFlyerState::FleeRight => {
            actor.position.move_x(3);
            actor.animate_two_frame_toggle(context.frame, 2);
            actor.position.move_y(-1);
            PerchedFlyerEffect::Flying {
                state: PerchedFlyerState::FleeRight,
            }
        }
        PerchedFlyerState::FleeLeft => {
            actor.position.move_x(-3);
            actor.animate_two_frame_toggle(context.frame, 2);
            actor.position.move_y(-1);
            PerchedFlyerEffect::Flying {
                state: PerchedFlyerState::FleeLeft,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perched_flyer_init_sets_type_and_perched_state() {
        let mut actor = Actor::default();
        let eff = update(
            &mut actor,
            PerchedFlyerContext {
                frame: FrameTick(0),
                mario_ref_x: 0,
            },
        );
        assert_eq!(eff, PerchedFlyerEffect::Initialized);
        assert_eq!(actor.contact_kind, ContactKind::from_raw(CONTACT_KIND));
        assert_eq!(actor.state, PerchedFlyerState::Perched.to_raw());
    }

    #[test]
    fn perched_flyer_waits_when_mario_is_far() {
        let mut actor = Actor::default();
        actor.position.x = 100;
        actor.state = PerchedFlyerState::Perched.to_raw();
        actor.sprite = 94;

        // Mario 30 px away horizontally (> 24)
        let eff = update(
            &mut actor,
            PerchedFlyerContext {
                frame: FrameTick(0),
                mario_ref_x: 130,
            },
        );
        assert_eq!(eff, PerchedFlyerEffect::None);
        assert_eq!(actor.state, PerchedFlyerState::Perched.to_raw());
        assert_eq!(actor.sprite, 95, "bit 0 set when perched");
        assert_eq!(actor.position.x, 100);
    }

    #[test]
    fn perched_flyer_startled_by_mario_on_right_flees_left() {
        let mut actor = Actor::default();
        actor.position.x = 100;
        actor.position.y = 80;
        actor.state = PerchedFlyerState::Perched.to_raw();
        actor.sprite = 94;

        // Mario 10 px away to the right (< 24)
        let eff = update(
            &mut actor,
            PerchedFlyerContext {
                frame: FrameTick(0),
                mario_ref_x: 110,
            },
        );
        assert_eq!(
            eff,
            PerchedFlyerEffect::Startled {
                sfx: STARTLE_SFX,
                facing: 1,
            }
        );
        assert_eq!(actor.state, PerchedFlyerState::FleeLeft.to_raw());
        assert!(actor.sprite_flip.horizontal());
        assert_eq!(actor.position.x, 100, "does not move on startle frame");

        // Next frame (frame 1, odd): moves X -3, Y -1, odd frame does not toggle sprite
        let eff1 = update(
            &mut actor,
            PerchedFlyerContext {
                frame: FrameTick(1),
                mario_ref_x: 110,
            },
        );
        assert_eq!(
            eff1,
            PerchedFlyerEffect::Flying {
                state: PerchedFlyerState::FleeLeft
            }
        );
        assert_eq!(actor.position.x, 97);
        assert_eq!(actor.position.y, 79);
        assert_eq!(actor.sprite, 95);

        // Next frame (frame 2, even): moves X -3, Y -1, toggles sprite
        let eff2 = update(
            &mut actor,
            PerchedFlyerContext {
                frame: FrameTick(2),
                mario_ref_x: 110,
            },
        );
        assert_eq!(
            eff2,
            PerchedFlyerEffect::Flying {
                state: PerchedFlyerState::FleeLeft
            }
        );
        assert_eq!(actor.position.x, 94);
        assert_eq!(actor.position.y, 78);
        assert_eq!(actor.sprite, 94);
    }

    #[test]
    fn perched_flyer_startled_by_mario_on_left_flees_right() {
        let mut actor = Actor::default();
        actor.position.x = 100;
        actor.position.y = 80;
        actor.state = PerchedFlyerState::Perched.to_raw();
        actor.sprite = 94;

        // Mario 10 px away to the left (< 24)
        let eff = update(
            &mut actor,
            PerchedFlyerContext {
                frame: FrameTick(0),
                mario_ref_x: 90,
            },
        );
        assert_eq!(
            eff,
            PerchedFlyerEffect::Startled {
                sfx: STARTLE_SFX,
                facing: 0xFF,
            }
        );
        assert_eq!(actor.state, PerchedFlyerState::FleeRight.to_raw());
        assert!(!actor.sprite_flip.horizontal());

        // Next frame: moves X +3, Y -1
        let eff1 = update(
            &mut actor,
            PerchedFlyerContext {
                frame: FrameTick(1),
                mario_ref_x: 90,
            },
        );
        assert_eq!(
            eff1,
            PerchedFlyerEffect::Flying {
                state: PerchedFlyerState::FleeRight
            }
        );
        assert_eq!(actor.position.x, 103);
        assert_eq!(actor.position.y, 79);
    }
}
