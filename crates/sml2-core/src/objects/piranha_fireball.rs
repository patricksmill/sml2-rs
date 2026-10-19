//! Native Fire Piranha Plant Fireball projectile domain behavior, ROM `$585ED` (anim 14).
//!
//! Follows the idiomatic Rust porting guide: typed actor state, domain methods,
//! zero raw hex offsets or register plumbing.

use crate::actor::{Actor, ContactKind, FrameTick};

/// Fireball movement state ($585ED).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FireballState {
    Init = 0,
    MovingLeft = 1,
    MovingRight = 2,
}

impl FireballState {
    pub const fn from_raw(value: u8) -> Self {
        match value {
            1 => Self::MovingLeft,
            2 => Self::MovingRight,
            _ => Self::Init,
        }
    }

    pub const fn to_raw(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FireballEffect {
    Initialized,
    Moving,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FireballContext {
    pub frame: FrameTick,
}

/// Domain update for Fire Piranha Plant Fireball (ROM $585ED, anim 14).
pub fn update(actor: &mut Actor, context: FireballContext) -> FireballEffect {
    // $585ED: toggle sprite bit 0 on every even frame (period 2).
    actor.animate_two_frame_toggle(context.frame, 2);

    let state = FireballState::from_raw(actor.state);
    match state {
        FireballState::Init => {
            // $585F8..$5860F: one-shot initialization.
            actor.contact_kind = ContactKind::from_raw(0x40);
            actor.position.move_y(-12);

            // If vertical flip is set (shot downwards), clear it and set downward Y direction (motion_leg = 1).
            if actor.sprite_flip.vertical() {
                actor.sprite_flip.set_vertical(false);
                actor.motion_leg = 1;
            }

            // $03CB1: set state based on horizontal flip.
            if actor.sprite_flip.horizontal() {
                actor.state = FireballState::MovingRight.to_raw();
            } else {
                actor.state = FireballState::MovingLeft.to_raw();
            }
            FireballEffect::Initialized
        }
        FireballState::MovingLeft => {
            // $58613: X -= 2
            actor.position.move_x(-2);
            // $5861F: step Y (motion_leg == 0 -> -1, motion_leg != 0 -> +1)
            step_y(actor);
            FireballEffect::Moving
        }
        FireballState::MovingRight => {
            // $5861A: X += 2
            actor.position.move_x(2);
            // $5861F: step Y
            step_y(actor);
            FireballEffect::Moving
        }
    }
}

#[inline]
fn step_y(actor: &mut Actor) {
    if actor.motion_leg != 0 {
        actor.position.move_y(1);
    } else {
        actor.position.move_y(-1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_upward_left() {
        let mut actor = Actor::default();
        actor.position.x = 100;
        actor.position.y = 200;
        actor.sprite = 22;
        // flip: neither horizontal nor vertical
        actor.sprite_flip.set_horizontal(false);
        actor.sprite_flip.set_vertical(false);

        let eff = update(
            &mut actor,
            FireballContext {
                frame: FrameTick(0),
            },
        );
        assert_eq!(eff, FireballEffect::Initialized);
        assert_eq!(actor.state, FireballState::MovingLeft.to_raw());
        assert_eq!(actor.contact_kind.raw(), 0x40);
        assert_eq!(actor.position.y, 188, "offset y by -12");
        assert_eq!(actor.position.x, 100);
        assert_eq!(actor.motion_leg, 0, "upward trajectory");
        assert_eq!(actor.sprite, 23, "frame 0 toggled sprite 22 -> 23");
    }

    #[test]
    fn test_init_downward_right() {
        let mut actor = Actor::default();
        actor.position.x = 100;
        actor.position.y = 200;
        actor.sprite_flip.set_horizontal(true);
        actor.sprite_flip.set_vertical(true);

        let eff = update(
            &mut actor,
            FireballContext {
                frame: FrameTick(1),
            },
        );
        assert_eq!(eff, FireballEffect::Initialized);
        assert_eq!(actor.state, FireballState::MovingRight.to_raw());
        assert_eq!(actor.contact_kind.raw(), 0x40);
        assert_eq!(actor.position.y, 188);
        assert_eq!(actor.motion_leg, 1, "downward trajectory");
        assert!(!actor.sprite_flip.vertical(), "vertical flip cleared");
        assert!(actor.sprite_flip.horizontal(), "horizontal flip preserved");
    }

    #[test]
    fn test_movement_upward_left() {
        let mut actor = Actor::default();
        actor.position.x = 100;
        actor.position.y = 188;
        actor.state = FireballState::MovingLeft.to_raw();
        actor.motion_leg = 0; // upward

        let eff = update(
            &mut actor,
            FireballContext {
                frame: FrameTick(1),
            },
        );
        assert_eq!(eff, FireballEffect::Moving);
        assert_eq!(actor.position.x, 98, "moved 2 px left");
        assert_eq!(actor.position.y, 187, "moved 1 px up");
    }

    #[test]
    fn test_movement_downward_right() {
        let mut actor = Actor::default();
        actor.position.x = 100;
        actor.position.y = 188;
        actor.state = FireballState::MovingRight.to_raw();
        actor.motion_leg = 1; // downward

        let eff = update(
            &mut actor,
            FireballContext {
                frame: FrameTick(1),
            },
        );
        assert_eq!(eff, FireballEffect::Moving);
        assert_eq!(actor.position.x, 102, "moved 2 px right");
        assert_eq!(actor.position.y, 189, "moved 1 px down");
    }

    #[test]
    fn test_sprite_toggling_every_2_frames() {
        let mut actor = Actor::default();
        actor.sprite = 22;
        actor.state = FireballState::MovingRight.to_raw();

        update(
            &mut actor,
            FireballContext {
                frame: FrameTick(1),
            },
        );
        assert_eq!(actor.sprite, 22, "odd frame: no toggle");

        update(
            &mut actor,
            FireballContext {
                frame: FrameTick(2),
            },
        );
        assert_eq!(actor.sprite, 23, "even frame: toggled");

        update(
            &mut actor,
            FireballContext {
                frame: FrameTick(3),
            },
        );
        assert_eq!(actor.sprite, 23, "odd frame: no toggle");

        update(
            &mut actor,
            FireballContext {
                frame: FrameTick(4),
            },
        );
        assert_eq!(actor.sprite, 22, "even frame: toggled back");
    }
}
