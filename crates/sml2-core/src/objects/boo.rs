//! Native Boo domain behavior, ROM `$59545` (anim 77).
//!
//! Follows the idiomatic Rust porting guide: typed actor state, domain methods,
//! zero raw hex offsets or register plumbing.

use crate::actor::{Actor, ContactKind, Facing, FrameTick};

/// Boo movement state ($59545).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooState {
    Shy = 0,
    ChasingRight = 1,
    ChasingLeft = 2,
}

impl BooState {
    pub const fn from_raw(value: u8) -> Self {
        match value {
            1 => Self::ChasingRight,
            2 => Self::ChasingLeft,
            _ => Self::Shy,
        }
    }

    pub const fn to_raw(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooEffect {
    Idle,
    Shy,
    Chasing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BooContext {
    pub frame: FrameTick,
    pub mario_dir_right: bool,
    pub mario_ref_x: u16,
    pub mario_ref_y: u16,
}

/// Signed Y-velocity profile from ROM `$59125` (17 entries, index 0..=16).
///
/// ROM bytes: `$FD, $FE, $FE, $FF, $FE, $FF, $FF, $00, $00, $00, $01, $01, $02, $01, $02, $02, $03`.
pub const BOO_Y_SPEED: [i8; 17] = [
    -3, -2, -2, -1, -2, -1, -1, 0,
    0,
    0, 1, 1, 2, 1, 2, 2, 3,
];

/// Derives the ROM `$5B3E0` facing relation from the object's perspective.
///
/// Returns 1 if Mario is to the right of the object, 255 if to the left, 0 if equal.
#[inline]
pub fn object_facing_to_mario(mario_ref_x: u16, actor_x: u16) -> u8 {
    if mario_ref_x > actor_x {
        1
    } else if mario_ref_x < actor_x {
        255
    } else {
        0
    }
}

/// Domain update for Boo (ROM $59545, anim 77).
pub fn update(
    actor: &mut Actor,
    context: BooContext,
) -> BooEffect {
    // $59545: Boo updates only every 4th frame.
    if !context.frame.every(4) {
        return BooEffect::Idle;
    }

    // $5954A: compare Mario's facing direction with Mario's relative position.
    // Mario is looking right when `sMarioDirection == 1`, and left when `sMarioDirection == 255`.
    let mario_dir = if context.mario_dir_right { 1u8 } else { 255u8 };
    let obj_facing = object_facing_to_mario(context.mario_ref_x, actor.position.x);

    // If Mario is facing toward Boo (mario_dir != obj_facing), Boo stops and covers its face.
    if mario_dir != obj_facing {
        // $5957F: sprite 14 (shy / covering face), state 0.
        actor.sprite = 14;
        actor.state = BooState::Shy.to_raw();
        return BooEffect::Shy;
    }

    // Mario has his back turned (mario_dir == obj_facing) -> Boo pursues Mario!
    let state = BooState::from_raw(actor.state);
    match state {
        BooState::Shy => {
            // $5955E..$5957E: uncover face, set pursuit state based on Mario's relative side.
            actor.sprite = 13;
            if obj_facing == 1 {
                // Mario is to the right
                actor.sprite_flip.set_horizontal(true);
                actor.state = BooState::ChasingRight.to_raw();
            } else {
                // Mario is to the left
                actor.sprite_flip.set_horizontal(false);
                actor.state = BooState::ChasingLeft.to_raw();
            }
            actor.motion_leg = 8; // neutral Y speed index
            actor.travel_direction = Facing::from_raw(8);
            actor.contact_kind = ContactKind::from_raw(0x48);
            BooEffect::Chasing
        }
        BooState::ChasingRight => {
            // $59587: update Y acceleration and step Y.
            step_y(actor, context.mario_ref_y);
            // Step X right (+1).
            actor.position.move_x(1);
            if actor.travel_direction.raw() < 16 {
                actor.travel_direction = Facing::from_raw(actor.travel_direction.raw().wrapping_add(1));
            }
            BooEffect::Chasing
        }
        BooState::ChasingLeft => {
            // $5959D: update Y acceleration and step Y.
            step_y(actor, context.mario_ref_y);
            // Step X left (-1).
            actor.position.move_x(-1);
            if actor.travel_direction.raw() > 0 {
                actor.travel_direction = Facing::from_raw(actor.travel_direction.raw().wrapping_sub(1));
            }
            BooEffect::Chasing
        }
    }
}

/// Helper for ROM `$590E9`: accelerate Y index toward Mario's Y, then step Y by the speed profile.
fn step_y(actor: &mut Actor, mario_ref_y: u16) {
    if mario_ref_y > actor.position.y {
        // Mario is below: accelerate downward (motion_leg increments toward 16)
        if actor.motion_leg < 16 {
            actor.motion_leg = actor.motion_leg.wrapping_add(1);
        }
    } else if mario_ref_y < actor.position.y {
        // Mario is above: accelerate upward (motion_leg decrements toward 0)
        if actor.motion_leg > 0 {
            actor.motion_leg = actor.motion_leg.wrapping_sub(1);
        }
    }
    let speed_idx = (actor.motion_leg as usize).min(BOO_Y_SPEED.len() - 1);
    let dy = BOO_Y_SPEED[speed_idx];
    actor.position.move_y(dy as i16);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_facing_to_mario() {
        assert_eq!(object_facing_to_mario(100, 80), 1);
        assert_eq!(object_facing_to_mario(80, 100), 255);
        assert_eq!(object_facing_to_mario(100, 100), 0);
    }

    #[test]
    fn test_frame_cadence_every_4() {
        let mut actor = Actor::default();
        let ctx = BooContext {
            frame: FrameTick(1), // not divisible by 4
            mario_dir_right: true,
            mario_ref_x: 200,
            mario_ref_y: 100,
        };
        assert_eq!(update(&mut actor, ctx), BooEffect::Idle);
    }

    #[test]
    fn test_mario_looking_at_boo_causes_shy_state() {
        let mut actor = Actor::default();
        actor.position.x = 100;
        actor.state = BooState::ChasingRight.to_raw();

        // Boo is at 100. Mario is at 150 (to the right) looking left (dir_right = false).
        // Mario is looking right at Boo!
        let ctx = BooContext {
            frame: FrameTick(4),
            mario_dir_right: false,
            mario_ref_x: 150,
            mario_ref_y: 100,
        };
        let eff = update(&mut actor, ctx);
        assert_eq!(eff, BooEffect::Shy);
        assert_eq!(actor.state, BooState::Shy.to_raw());
        assert_eq!(actor.sprite, 14, "shy sprite is 14");
    }

    #[test]
    fn test_mario_turned_away_triggers_chase_right() {
        let mut actor = Actor::default();
        actor.position.x = 100;
        actor.position.y = 100;
        actor.state = BooState::Shy.to_raw();

        // Mario is at 150 (to the right) facing right (turned away!).
        let ctx = BooContext {
            frame: FrameTick(4),
            mario_dir_right: true,
            mario_ref_x: 150,
            mario_ref_y: 100,
        };
        let eff = update(&mut actor, ctx);
        assert_eq!(eff, BooEffect::Chasing);
        assert_eq!(actor.state, BooState::ChasingRight.to_raw());
        assert_eq!(actor.sprite, 13);
        assert!(actor.sprite_flip.horizontal());
        assert_eq!(actor.motion_leg, 8);
        assert_eq!(actor.travel_direction.raw(), 8);
        assert_eq!(actor.contact_kind.raw(), 0x48);

        // Next 4th frame: advances right in X, keeps tracking Y
        let ctx = BooContext {
            frame: FrameTick(8),
            mario_dir_right: true,
            mario_ref_x: 150,
            mario_ref_y: 150, // Mario is below -> motion_leg accelerates to 9
        };
        let eff = update(&mut actor, ctx);
        assert_eq!(eff, BooEffect::Chasing);
        assert_eq!(actor.position.x, 101);
        assert_eq!(actor.motion_leg, 9);
        assert_eq!(actor.travel_direction.raw(), 9);
    }

    #[test]
    fn test_mario_turned_away_triggers_chase_left() {
        let mut actor = Actor::default();
        actor.position.x = 100;
        actor.position.y = 100;
        actor.state = BooState::Shy.to_raw();

        // Mario is at 50 (to the left) facing left (turned away!).
        let ctx = BooContext {
            frame: FrameTick(4),
            mario_dir_right: false,
            mario_ref_x: 50,
            mario_ref_y: 50, // Mario is above
        };
        let eff = update(&mut actor, ctx);
        assert_eq!(eff, BooEffect::Chasing);
        assert_eq!(actor.state, BooState::ChasingLeft.to_raw());
        assert_eq!(actor.sprite, 13);
        assert!(!actor.sprite_flip.horizontal());

        // Next step: moves left
        let ctx = BooContext {
            frame: FrameTick(8),
            mario_dir_right: false,
            mario_ref_x: 50,
            mario_ref_y: 50,
        };
        update(&mut actor, ctx);
        assert_eq!(actor.position.x, 99);
        assert_eq!(actor.motion_leg, 7); // accelerated upward toward index 0
        assert_eq!(actor.travel_direction.raw(), 7);
    }
}
