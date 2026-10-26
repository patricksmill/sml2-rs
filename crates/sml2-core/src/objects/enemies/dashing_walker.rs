//! Native Dashing Walker domain behavior, ROM `$59D5A` (anim 6 Mode B).
//!
//! Follows the idiomatic Rust porting guide: typed actor state, domain methods,
//! zero raw hex offsets or register plumbing.

use super::pipe_bobber::dist_mario_x;
use super::staging::{BOUNCE_SPEED, FALL_SPEED};
use crate::actor::{Actor, ContactKind, FlashFlags, FrameTick};

/// Dashing Walker state machine (ROM $59D5A).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashingWalkerState {
    Init = 0,
    PatrolLeft = 1,
    PatrolRight = 2,
    ChargeLeft = 3,
    ChargeRight = 4,
}

impl DashingWalkerState {
    pub const fn from_raw(value: u8) -> Self {
        match value {
            1 => Self::PatrolLeft,
            2 => Self::PatrolRight,
            3 => Self::ChargeLeft,
            4 => Self::ChargeRight,
            _ => Self::Init,
        }
    }

    pub const fn to_raw(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashingWalkerEffect {
    Initialized { facing: u8 },
    Patrolling,
    Charging,
    Turnaround,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DashingWalkerContext {
    pub frame: FrameTick,
    pub mario_ref_x: u16,
}

/// World probing interface for Dashing Walker wall collision.
pub trait DashingWalkerWorld {
    fn probe_left_wall(&mut self, actor: &Actor) -> bool;
    fn probe_right_wall(&mut self, actor: &Actor) -> bool;
}

/// Fall/charge step helper ($5B449 / $5B481): reads from `FALL_SPEED` or `BOUNCE_SPEED`
/// capped at 15, and advances `motion_phase` up to 15.
#[inline]
pub fn charge_speed_step(actor: &mut Actor, is_decel: bool) -> (u8, bool) {
    let phase = (actor.motion_phase as usize).min(15);
    let speed = if is_decel {
        BOUNCE_SPEED[phase]
    } else {
        FALL_SPEED[phase]
    };
    if actor.motion_phase < 15 {
        actor.motion_phase = actor.motion_phase.wrapping_add(1);
    }
    let at_cap = actor.motion_phase == 15;
    (speed, at_cap)
}

/// Domain update for Dashing Walker (ROM $59D5A, anim 6).
pub fn update<W: DashingWalkerWorld>(
    actor: &mut Actor,
    context: DashingWalkerContext,
    world: &mut W,
) -> DashingWalkerEffect {
    // $59D5A: 2-frame sprite toggle on every 4th frame ($5B51A)
    actor.animate_two_frame_toggle(context.frame, 4);

    let state = DashingWalkerState::from_raw(actor.state);
    match state {
        DashingWalkerState::Init => {
            // $59D62..$59D6A: flash $10, contact kind 72 (0x48), face toward Mario
            actor.flash = FlashFlags::from_bits(0x10);
            actor.contact_kind = ContactKind::from_raw(72);
            let facing = if context.mario_ref_x > actor.position.x {
                // Mario to the right -> PatrolRight (2)
                actor.state = DashingWalkerState::PatrolRight.to_raw();
                actor.sprite_flip.set_horizontal(true);
                1
            } else if context.mario_ref_x < actor.position.x {
                // Mario to the left -> PatrolLeft (1)
                actor.state = DashingWalkerState::PatrolLeft.to_raw();
                actor.sprite_flip.set_horizontal(false);
                0xFF
            } else {
                actor.state = DashingWalkerState::PatrolLeft.to_raw();
                actor.sprite_flip.set_horizontal(false);
                0
            };
            DashingWalkerEffect::Initialized { facing }
        }
        DashingWalkerState::PatrolLeft => {
            // $59D7A: check left wall collision
            if world.probe_left_wall(actor) {
                // $59D2D: hit wall -> turn around to PatrolRight (2)
                actor.sprite_flip.toggle_horizontal();
                actor.state = DashingWalkerState::PatrolRight.to_raw();
                return DashingWalkerEffect::Turnaround;
            }

            // $59D84: step 1 px left
            actor.position.move_x(-1);

            // $59D9B: check proximity to Mario (< 36 px / 0x24)
            if dist_mario_x(context.mario_ref_x, actor.position.x) < 0x24 {
                // $59DA1: trigger ChargeLeft (3)
                actor.state = DashingWalkerState::ChargeLeft.to_raw();
                actor.motion_phase = 0;
                actor.set_motion_counter(0);
            }
            DashingWalkerEffect::Patrolling
        }
        DashingWalkerState::PatrolRight => {
            // $59D8B: check right wall collision
            if world.probe_right_wall(actor) {
                // $59D2D: hit wall -> turn around to PatrolLeft (1)
                actor.sprite_flip.toggle_horizontal();
                actor.state = DashingWalkerState::PatrolLeft.to_raw();
                return DashingWalkerEffect::Turnaround;
            }

            // $59D96: step 1 px right
            actor.position.move_x(1);

            // $59D9B: check proximity to Mario (< 36 px / 0x24)
            if dist_mario_x(context.mario_ref_x, actor.position.x) < 0x24 {
                // $59DA1: trigger ChargeRight (4)
                actor.state = DashingWalkerState::ChargeRight.to_raw();
                actor.motion_phase = 0;
                actor.set_motion_counter(0);
            }
            DashingWalkerEffect::Patrolling
        }
        DashingWalkerState::ChargeLeft => {
            // $59DA8: check left wall collision during charge
            if world.probe_left_wall(actor) {
                // $59DF9: bounce off wall -> turn around to ChargeRight (4)
                actor.sprite_flip.toggle_horizontal();
                actor.state = DashingWalkerState::ChargeRight.to_raw();
                return DashingWalkerEffect::Turnaround;
            }

            let is_decel = actor.motion_counter() != 0;
            let (speed, at_cap) = charge_speed_step(actor, is_decel);
            actor.position.move_x(-(speed as i16));

            if at_cap {
                actor.motion_phase = 0;
                actor.set_motion_counter(actor.motion_counter() ^ 1);
                if actor.motion_counter() == 0 {
                    // Both accel and decel legs finished: resume PatrolLeft (1)
                    actor.state = DashingWalkerState::PatrolLeft.to_raw();
                }
            }
            DashingWalkerEffect::Charging
        }
        DashingWalkerState::ChargeRight => {
            // $59DC7: check right wall collision during charge
            if world.probe_right_wall(actor) {
                // $59DF9: bounce off wall -> turn around to ChargeLeft (3)
                actor.sprite_flip.toggle_horizontal();
                actor.state = DashingWalkerState::ChargeLeft.to_raw();
                return DashingWalkerEffect::Turnaround;
            }

            let is_decel = actor.motion_counter() != 0;
            let (speed, at_cap) = charge_speed_step(actor, is_decel);
            actor.position.move_x(speed as i16);

            if at_cap {
                actor.motion_phase = 0;
                actor.set_motion_counter(actor.motion_counter() ^ 1);
                if actor.motion_counter() == 0 {
                    // Both accel and decel legs finished: resume PatrolRight (2)
                    actor.state = DashingWalkerState::PatrolRight.to_raw();
                }
            }
            DashingWalkerEffect::Charging
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockWorld {
        pub hit_left: bool,
        pub hit_right: bool,
    }

    impl DashingWalkerWorld for MockWorld {
        fn probe_left_wall(&mut self, _actor: &Actor) -> bool {
            self.hit_left
        }
        fn probe_right_wall(&mut self, _actor: &Actor) -> bool {
            self.hit_right
        }
    }

    #[test]
    fn test_init_facing_and_flags() {
        let mut actor = Actor::default();
        actor.position.x = 100;
        actor.position.y = 80;
        actor.sprite = 14;

        let mut world = MockWorld {
            hit_left: false,
            hit_right: false,
        };

        // Mario is to the right (x = 150 > 100) -> facing 1, PatrolRight (2)
        let eff = update(
            &mut actor,
            DashingWalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 150,
            },
            &mut world,
        );
        assert_eq!(eff, DashingWalkerEffect::Initialized { facing: 1 });
        assert_eq!(actor.state, DashingWalkerState::PatrolRight.to_raw());
        assert!(actor.sprite_flip.horizontal());
        assert_eq!(actor.contact_kind.raw(), 72);
        assert_eq!(actor.flash.bits(), 0x10);

        // Mario is to the left (x = 50 < 100) -> facing 0xFF, PatrolLeft (1)
        actor.state = 0;
        let eff_left = update(
            &mut actor,
            DashingWalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 50,
            },
            &mut world,
        );
        assert_eq!(eff_left, DashingWalkerEffect::Initialized { facing: 0xFF });
        assert_eq!(actor.state, DashingWalkerState::PatrolLeft.to_raw());
        assert!(!actor.sprite_flip.horizontal());

        // Mario at same X (x = 100) -> facing 0, PatrolLeft (1)
        actor.state = 0;
        let eff_eq = update(
            &mut actor,
            DashingWalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 100,
            },
            &mut world,
        );
        assert_eq!(eff_eq, DashingWalkerEffect::Initialized { facing: 0 });
        assert_eq!(actor.state, DashingWalkerState::PatrolLeft.to_raw());
        assert!(!actor.sprite_flip.horizontal());
    }

    #[test]
    fn test_patrol_movement_and_wall_turnaround() {
        let mut actor = Actor::default();
        actor.position.x = 100;
        actor.position.y = 80;
        actor.state = DashingWalkerState::PatrolLeft.to_raw();
        actor.sprite_flip.set_horizontal(false);

        let mut world = MockWorld {
            hit_left: false,
            hit_right: false,
        };

        // Mario far away: x = 200 (dx = 100 >= 36) -> steps 1 px left
        let eff = update(
            &mut actor,
            DashingWalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 200,
            },
            &mut world,
        );
        assert_eq!(eff, DashingWalkerEffect::Patrolling);
        assert_eq!(actor.position.x, 99);

        // Hits left wall -> turns around to PatrolRight (2) and sets horizontal flip
        world.hit_left = true;
        let eff2 = update(
            &mut actor,
            DashingWalkerContext {
                frame: FrameTick(2),
                mario_ref_x: 200,
            },
            &mut world,
        );
        assert_eq!(eff2, DashingWalkerEffect::Turnaround);
        assert_eq!(actor.state, DashingWalkerState::PatrolRight.to_raw());
        assert!(actor.sprite_flip.horizontal());
        assert_eq!(actor.position.x, 99, "does not move on turnaround frame");
    }

    #[test]
    fn test_proximity_triggers_charge() {
        let mut actor = Actor::default();
        actor.position.x = 100;
        actor.position.y = 80;
        actor.state = DashingWalkerState::PatrolLeft.to_raw();

        let mut world = MockWorld {
            hit_left: false,
            hit_right: false,
        };

        // Mario within 30 px (< 36) -> steps left and transitions to ChargeLeft (3)
        let eff = update(
            &mut actor,
            DashingWalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 130,
            },
            &mut world,
        );
        assert_eq!(eff, DashingWalkerEffect::Patrolling);
        assert_eq!(actor.position.x, 99);
        assert_eq!(actor.state, DashingWalkerState::ChargeLeft.to_raw());
        assert_eq!(actor.motion_phase, 0);
        assert_eq!(actor.motion_counter(), 0);
    }

    #[test]
    fn test_full_charge_cycle_accel_decel_and_resume_patrol() {
        let mut actor = Actor::default();
        actor.position.x = 200;
        actor.position.y = 80;
        actor.state = DashingWalkerState::ChargeLeft.to_raw();
        actor.motion_phase = 0;
        actor.set_motion_counter(0);

        let mut world = MockWorld {
            hit_left: false,
            hit_right: false,
        };

        // Leg 1 (Accel): 15 steps (phases 0..14; at step 15 phase reaches 15, resets to 0, switches to deceleration)
        for phase in 0..15 {
            let eff = update(
                &mut actor,
                DashingWalkerContext {
                    frame: FrameTick(phase as u64),
                    mario_ref_x: 0,
                },
                &mut world,
            );
            assert_eq!(eff, DashingWalkerEffect::Charging);
        }
        assert_eq!(
            actor.motion_phase, 0,
            "resets phase to 0 on finishing accel leg"
        );
        assert_eq!(actor.motion_counter(), 1, "switches to deceleration");
        assert_eq!(actor.state, DashingWalkerState::ChargeLeft.to_raw());

        // Leg 2 (Decel): 15 steps
        for step in 0..15 {
            let eff = update(
                &mut actor,
                DashingWalkerContext {
                    frame: FrameTick(15 + step),
                    mario_ref_x: 0,
                },
                &mut world,
            );
            assert_eq!(eff, DashingWalkerEffect::Charging);
        }
        assert_eq!(
            actor.motion_phase, 0,
            "resets phase to 0 on finishing decel leg"
        );
        assert_eq!(actor.motion_counter(), 0, "switches back to acceleration");
        assert_eq!(
            actor.state,
            DashingWalkerState::PatrolLeft.to_raw(),
            "resumed patrol after full charge cycle"
        );
    }

    #[test]
    fn charge_phase_toggle_preserves_unknown_counter_bits() {
        for state in [3, 4] {
            for count in 0..=255 {
                let mut slot = [0; 32];
                slot[15] = state;
                slot[16] = 14;
                slot[25] = count;
                let mut actor = Actor::from_raw_slot(&slot);
                let mut world = MockWorld {
                    hit_left: false,
                    hit_right: false,
                };
                update(
                    &mut actor,
                    DashingWalkerContext {
                        frame: FrameTick(1),
                        mario_ref_x: 0,
                    },
                    &mut world,
                );
                let result = actor.to_raw_slot();
                assert_eq!(result[25], count ^ 1);
                assert_eq!(result[16], 0);
                assert_eq!(result[15], if count == 1 { state - 2 } else { state });
            }
        }
    }

    #[test]
    fn test_charge_wall_bounce() {
        let mut actor = Actor::default();
        actor.position.x = 200;
        actor.position.y = 80;
        actor.state = DashingWalkerState::ChargeLeft.to_raw();
        actor.sprite_flip.set_horizontal(false);

        let mut world = MockWorld {
            hit_left: true,
            hit_right: false,
        };

        // Hits wall while charging left -> bounces to ChargeRight (4) and flips horizontally
        let eff = update(
            &mut actor,
            DashingWalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 0,
            },
            &mut world,
        );
        assert_eq!(eff, DashingWalkerEffect::Turnaround);
        assert_eq!(actor.state, DashingWalkerState::ChargeRight.to_raw());
        assert!(actor.sprite_flip.horizontal());
    }
}
