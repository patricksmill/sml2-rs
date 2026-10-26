//! Native Boo domain behavior, ROM `$59545` (anim 77).
//!
//! Follows the idiomatic Rust porting guide: typed actor state, domain methods,
//! zero raw hex offsets or register plumbing.

use crate::actor::{Actor, ContactKind, FrameTick, SpriteFlipFlags, VerticalTrackingError};

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
pub fn update(actor: &mut Actor, context: BooContext) -> Result<BooEffect, VerticalTrackingError> {
    // $59545: Boo updates only every 4th frame.
    if !context.frame.every(4) {
        return Ok(BooEffect::Idle);
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
        return Ok(BooEffect::Shy);
    }

    // Mario has his back turned (mario_dir == obj_facing) -> Boo pursues Mario!
    let state = BooState::from_raw(actor.state);
    Ok(match state {
        BooState::Shy => {
            // $5955E..$5957E: uncover face, set pursuit state based on Mario's relative side.
            actor.sprite = 13;
            if obj_facing == 1 {
                // Mario is to the right
                actor.sprite_flip = SpriteFlipFlags::from_bits(32);
                actor.state = BooState::ChasingRight.to_raw();
            } else {
                // Mario is to the left
                actor.sprite_flip = SpriteFlipFlags::from_bits(0);
                actor.state = BooState::ChasingLeft.to_raw();
            }
            actor.motion_leg = 8; // neutral Y speed index
            actor.set_motion_counter(8);
            actor.contact_kind = ContactKind::from_raw(0x48);
            BooEffect::Chasing
        }
        BooState::ChasingRight => {
            // $59587: update Y acceleration and step Y.
            actor.track_player_y(context.mario_ref_y)?;
            // Step X right (+1).
            actor.position.move_x(1);
            if actor.motion_counter() != 16 {
                actor.set_motion_counter(actor.motion_counter().wrapping_add(1));
            }
            BooEffect::Chasing
        }
        BooState::ChasingLeft => {
            // $5959D: update Y acceleration and step Y.
            actor.track_player_y(context.mario_ref_y)?;
            // Step X left (-1).
            actor.position.move_x(-1);
            if actor.motion_counter() > 0 {
                actor.set_motion_counter(actor.motion_counter().wrapping_sub(1));
            }
            BooEffect::Chasing
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boo_source_initialization_overwrites_dirty_sprite_flags() {
        for (right, flip, state) in [(true, 32, 1), (false, 0, 2)] {
            for dirty in [0xff, 0xdf, 0x40, 0x1f] {
                let mut expected = std::array::from_fn(|i| (i as u8).wrapping_mul(37));
                expected[0..2].copy_from_slice(&100u16.to_be_bytes());
                expected[3..5].copy_from_slice(&100u16.to_be_bytes());
                expected[13] = dirty;
                expected[15] = 0;
                let mut actor = Actor::from_raw_slot(&expected);
                assert_eq!(
                    update(
                        &mut actor,
                        BooContext {
                            frame: FrameTick(4),
                            mario_dir_right: right,
                            mario_ref_x: if right { 150 } else { 50 },
                            mario_ref_y: 100,
                        }
                    ),
                    Ok(BooEffect::Chasing)
                );
                expected[11] = 13;
                expected[13] = flip;
                expected[15] = state;
                expected[18] = 0x48;
                expected[24] = 8;
                expected[25] = 8;
                assert_eq!(actor.to_raw_slot(), expected);
            }
        }
    }

    #[test]
    fn boo_source_travel_counter_exact_endpoints_and_byte_wrap() {
        for (right, counter, next) in [
            (true, 15, 16),
            (true, 16, 16),
            (true, 17, 18),
            (true, 255, 0),
            (false, 0, 0),
            (false, 1, 0),
            (false, 255, 254),
        ] {
            let mut expected = std::array::from_fn(|i| (i as u8).wrapping_mul(37));
            expected[0..2].copy_from_slice(&100u16.to_be_bytes());
            expected[3..5].copy_from_slice(&100u16.to_be_bytes());
            expected[15] = if right { 1 } else { 2 };
            expected[24] = 8;
            expected[25] = counter;
            let mut actor = Actor::from_raw_slot(&expected);
            assert_eq!(
                update(
                    &mut actor,
                    BooContext {
                        frame: FrameTick(4),
                        mario_dir_right: right,
                        mario_ref_x: if right { 150 } else { 50 },
                        mario_ref_y: 100,
                    }
                ),
                Ok(BooEffect::Chasing)
            );
            expected[1] = if right { 101 } else { 99 };
            expected[24] = 9; // Equality increments Y tracking; profile byte 9 is zero.
            expected[25] = next;
            assert_eq!(actor.to_raw_slot(), expected);
        }
    }

    #[test]
    fn boo_adapter_projects_source_facing_and_profile_errors() {
        use super::super::{ObjectSim, Staging};
        for (frame, state, player_x, player_y, leg, facing, error) in [
            (1, 1, 150, 50, 9, 42, false),
            (4, 0, 150, 50, 9, 1, false),
            (4, 1, 50, 50, 9, 255, false),
            (4, 1, 150, 50, 9, 255, false),
            (4, 1, 150, 100, 9, 0, false),
            (4, 1, 150, 101, 9, 1, false),
            (4, 1, 150, 100, 17, 0, true),
        ] {
            let mut sim = ObjectSim::new(Vec::new(), 0);
            sim.fff0 = frame;
            sim.facing = 42;
            sim.mario.dir_right = true;
            sim.mario_ref_x = player_x;
            sim.mario_ref_y = player_y;
            let mut actor = Actor::default();
            actor.position.x = 100;
            actor.position.y = 100;
            actor.state = state;
            actor.motion_leg = leg;
            let mut staging = Staging::default();
            staging.b = actor.to_raw_slot();
            sim.handler_boo(&mut staging);
            assert_eq!(sim.facing, facing);
            assert_eq!(sim.unported.contains(&0x59545), error);
            if error {
                actor.motion_leg = 18;
                assert_eq!(staging.b, actor.to_raw_slot());
            }
        }
    }

    #[test]
    fn boo_tracking_uses_shared_rom_equality_and_propagates_profile_error() {
        assert_eq!(crate::actor::VERTICAL_TRACKING_Y_SPEED[0], -3);
        for (leg, next, y, result) in [
            (9, 10, 101u16, Ok(BooEffect::Chasing)),
            (255, 0, 97, Ok(BooEffect::Chasing)),
            (17, 18, 100, Err(VerticalTrackingError { leg: 18 })),
        ] {
            let mut expected = [0xa5; 32];
            expected[0..2].copy_from_slice(&100u16.to_be_bytes());
            expected[3..5].copy_from_slice(&100u16.to_be_bytes());
            expected[15] = 1;
            expected[24] = leg;
            expected[25] = 8;
            let mut actor = Actor::from_raw_slot(&expected);
            assert_eq!(
                update(
                    &mut actor,
                    BooContext {
                        frame: FrameTick(4),
                        mario_dir_right: true,
                        mario_ref_x: 150,
                        mario_ref_y: 100,
                    }
                ),
                result
            );
            expected[24] = next;
            expected[3..5].copy_from_slice(&y.to_be_bytes());
            if result.is_ok() {
                expected[1] = 101;
                expected[25] = 9;
            }
            assert_eq!(actor.to_raw_slot(), expected);
        }
    }

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
        assert_eq!(update(&mut actor, ctx), Ok(BooEffect::Idle));
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
        assert_eq!(eff, Ok(BooEffect::Shy));
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
        assert_eq!(eff, Ok(BooEffect::Chasing));
        assert_eq!(actor.state, BooState::ChasingRight.to_raw());
        assert_eq!(actor.sprite, 13);
        assert!(actor.sprite_flip.horizontal());
        assert_eq!(actor.motion_leg, 8);
        assert_eq!(actor.motion_counter(), 8);
        assert_eq!(actor.contact_kind.raw(), 0x48);

        // Next 4th frame: advances right in X, keeps tracking Y
        let ctx = BooContext {
            frame: FrameTick(8),
            mario_dir_right: true,
            mario_ref_x: 150,
            mario_ref_y: 150, // Mario is below -> motion_leg accelerates to 9
        };
        let eff = update(&mut actor, ctx);
        assert_eq!(eff, Ok(BooEffect::Chasing));
        assert_eq!(actor.position.x, 101);
        assert_eq!(actor.motion_leg, 9);
        assert_eq!(actor.motion_counter(), 9);
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
        assert_eq!(eff, Ok(BooEffect::Chasing));
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
        update(&mut actor, ctx).unwrap();
        assert_eq!(actor.position.x, 99);
        assert_eq!(actor.motion_leg, 7); // accelerated upward toward index 0
        assert_eq!(actor.motion_counter(), 7);
    }
}
