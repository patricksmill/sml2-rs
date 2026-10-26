//! Native Fire Piranha Plant (Venus Fire Trap) domain behavior, ROM `$58541` (anim 13).
//!
//! Follows the idiomatic Rust porting guide: typed actor state, domain methods,
//! zero raw hex offsets or register plumbing.

use super::pipe_bobber::dist_mario_x;
use crate::actor::{Actor, ContactKind, FrameTick, SpriteFlipFlags};

/// Fire Piranha Plant state machine ($58541).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirePiranhaState {
    Waiting = 0,
    Emerging = 1,
    Shooting = 2,
    Retracting = 3,
    Resting = 4,
}

impl FirePiranhaState {
    pub const fn from_raw(value: u8) -> Self {
        match value {
            1 => Self::Emerging,
            2 => Self::Shooting,
            3 => Self::Retracting,
            4 => Self::Resting,
            _ => Self::Waiting,
        }
    }

    pub const fn to_raw(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirePiranhaEffect {
    Waiting,
    Emerging,
    SpawnProjectile { flip: SpriteFlipFlags },
    Shooting,
    Retracting,
    Resting,
    Despawn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirePiranhaContext {
    pub frame: FrameTick,
    pub mario_travelling: bool,
    pub mario_ref_x: u16,
    pub mario_ref_y: u16,
}

/// Helper for ROM `$585DA` / `$5B41A`: update sprite to 18 (aiming up) or 20 (aiming down).
#[inline]
pub fn update_vertical_aim_sprite(actor: &mut Actor, mario_ref_y: u16) {
    if mario_ref_y > actor.position.y {
        actor.sprite = 20;
    } else {
        actor.sprite = 18;
    }
}

/// Domain update for Fire Piranha Plant (ROM $58541, anim 13).
pub fn update(actor: &mut Actor, context: FirePiranhaContext) -> FirePiranhaEffect {
    // ROM $58541: pipe travel check.
    if context.mario_travelling {
        return FirePiranhaEffect::Despawn;
    }

    let state = FirePiranhaState::from_raw(actor.state);
    match state {
        FirePiranhaState::Waiting => {
            // $5854E: face toward Mario in X.
            // If Mario is to the right (mario_ref_x >= actor.x), set horizontal flip bit ($20).
            actor
                .sprite_flip
                .set_horizontal(context.mario_ref_x >= actor.position.x);

            // $584F3: defeat mode $80 (invulnerable/inactive) while waiting in pipe.
            actor.defeat_mode = 0x80;
            if dist_mario_x(context.mario_ref_x, actor.position.x) < 0x18 {
                return FirePiranhaEffect::Waiting;
            }
            actor.state = FirePiranhaState::Emerging.to_raw();
            actor.motion_leg = 32;
            actor.set_travel_span(32);
            actor.contact_kind = ContactKind::from_raw(0x44);
            FirePiranhaEffect::Emerging
        }
        FirePiranhaState::Emerging => {
            // $58566: update aim sprite (18 or 20) and rise (y -= 1).
            update_vertical_aim_sprite(actor, context.mario_ref_y);
            actor.position.move_y(-1);
            actor.motion_leg = actor.motion_leg.wrapping_sub(1);
            if actor.motion_leg != 0 {
                return FirePiranhaEffect::Emerging;
            }

            // $58574..$58596: finished rising. Open mouth and spawn child projectile (anim 14).
            actor.sprite = actor.sprite.wrapping_add(1); // 18 -> 19, or 20 -> 21
            let child_flip = SpriteFlipFlags::from_bits(
                actor.sprite_flip.bits() | if actor.sprite != 19 { 0x40 } else { 0 },
            );
            actor.state = FirePiranhaState::Shooting.to_raw();
            actor.timer = 48;
            FirePiranhaEffect::SpawnProjectile { flip: child_flip }
        }
        FirePiranhaState::Shooting => {
            // $585A2: wait 48 frames with mouth open.
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer != 0 {
                return FirePiranhaEffect::Shooting;
            }

            // $585A8: timer reached zero. Close mouth and begin retracting.
            actor.state = FirePiranhaState::Retracting.to_raw();
            actor.motion_leg = actor.travel_span(); // 32
            actor.sprite = actor.sprite.wrapping_sub(1); // 19 -> 18, 21 -> 20
            FirePiranhaEffect::Retracting
        }
        FirePiranhaState::Retracting => {
            // $585B6: update aim sprite (18 or 20) and sink (y += 1).
            update_vertical_aim_sprite(actor, context.mario_ref_y);
            actor.position.move_y(1);
            actor.motion_leg = actor.motion_leg.wrapping_sub(1);
            if actor.motion_leg != 0 {
                return FirePiranhaEffect::Retracting;
            }

            // $585C4: reached bottom. Enter resting state inside pipe.
            actor.state = FirePiranhaState::Resting.to_raw();
            actor.timer = 4;
            FirePiranhaEffect::Resting
        }
        FirePiranhaState::Resting => {
            // $585CB: decrement timer every 8th frame.
            if !context.frame.every(8) {
                return FirePiranhaEffect::Resting;
            }
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer != 0 {
                return FirePiranhaEffect::Resting;
            }

            // $585D6: rest complete. Return to waiting state.
            actor.state = FirePiranhaState::Waiting.to_raw();
            FirePiranhaEffect::Waiting
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_waiting_facing_and_distance_gate() {
        let mut actor = Actor::default();
        actor.position.x = 100;

        // Mario to left (x = 90), distance 10 < 24
        let ctx = FirePiranhaContext {
            frame: FrameTick(0),
            mario_travelling: false,
            mario_ref_x: 90,
            mario_ref_y: 100,
        };
        let eff = update(&mut actor, ctx);
        assert_eq!(eff, FirePiranhaEffect::Waiting);
        assert_eq!(actor.state, FirePiranhaState::Waiting.to_raw());
        assert!(!actor.sprite_flip.horizontal());
        assert_eq!(actor.defeat_mode, 0x80);

        // Mario to right (x = 110), distance 10 < 24
        let ctx = FirePiranhaContext {
            frame: FrameTick(0),
            mario_travelling: false,
            mario_ref_x: 110,
            mario_ref_y: 100,
        };
        let eff = update(&mut actor, ctx);
        assert_eq!(eff, FirePiranhaEffect::Waiting);
        assert!(actor.sprite_flip.horizontal());

        // Mario far enough (x = 124, distance 24 >= 24)
        let ctx = FirePiranhaContext {
            frame: FrameTick(0),
            mario_travelling: false,
            mario_ref_x: 124,
            mario_ref_y: 100,
        };
        let eff = update(&mut actor, ctx);
        assert_eq!(eff, FirePiranhaEffect::Emerging);
        assert_eq!(actor.state, FirePiranhaState::Emerging.to_raw());
        assert_eq!(actor.motion_leg, 32);
        assert_eq!(actor.travel_span(), 32);
        assert_eq!(actor.contact_kind.raw(), 0x44);
    }

    #[test]
    fn test_aiming_up_and_spawning_projectile() {
        let mut actor = Actor::default();
        actor.position.x = 100;
        actor.position.y = 200;
        actor.state = FirePiranhaState::Emerging.to_raw();
        actor.motion_leg = 32;
        actor.set_travel_span(32);
        actor.sprite_flip.set_horizontal(true);

        // Mario above (mario_ref_y = 150 < actor.y)
        for i in 1..=31 {
            let ctx = FirePiranhaContext {
                frame: FrameTick(i),
                mario_travelling: false,
                mario_ref_x: 150,
                mario_ref_y: 150,
            };
            let eff = update(&mut actor, ctx);
            assert_eq!(eff, FirePiranhaEffect::Emerging);
            assert_eq!(actor.sprite, 18, "aiming up sprite is 18");
            assert_eq!(actor.position.y, 200 - i as u16);
            assert_eq!(actor.motion_leg, (32 - i) as u8);
        }

        // Step 32: finishes rising, opens mouth (sprite 19), spawns projectile
        let ctx = FirePiranhaContext {
            frame: FrameTick(32),
            mario_travelling: false,
            mario_ref_x: 150,
            mario_ref_y: 150,
        };
        let eff = update(&mut actor, ctx);
        assert_eq!(actor.sprite, 19, "open mouth upward sprite is 19");
        assert_eq!(actor.state, FirePiranhaState::Shooting.to_raw());
        assert_eq!(actor.timer, 48);
        if let FirePiranhaEffect::SpawnProjectile { flip } = eff {
            assert!(flip.horizontal());
            assert!(!flip.vertical(), "aiming up does not set vertical flip");
        } else {
            panic!("expected SpawnProjectile effect");
        }
    }

    #[test]
    fn test_aiming_down_and_spawning_projectile_with_vflip() {
        let mut actor = Actor::default();
        actor.position.x = 100;
        actor.position.y = 200;
        actor.state = FirePiranhaState::Emerging.to_raw();
        actor.motion_leg = 1; // 1 step left
        actor.set_travel_span(32);

        // Mario below (mario_ref_y = 250 > actor.y)
        let ctx = FirePiranhaContext {
            frame: FrameTick(1),
            mario_travelling: false,
            mario_ref_x: 150,
            mario_ref_y: 250,
        };
        let eff = update(&mut actor, ctx);
        assert_eq!(actor.sprite, 21, "open mouth downward sprite is 21");
        assert_eq!(actor.state, FirePiranhaState::Shooting.to_raw());
        assert_eq!(actor.timer, 48);
        if let FirePiranhaEffect::SpawnProjectile { flip } = eff {
            assert!(
                flip.vertical(),
                "aiming down sets vertical flip on projectile"
            );
        } else {
            panic!("expected SpawnProjectile effect");
        }
    }

    #[test]
    fn test_shooting_pause_and_retraction_and_rest() {
        let mut actor = Actor::default();
        actor.position.x = 100;
        actor.position.y = 168; // emerged 32 px from 200
        actor.state = FirePiranhaState::Shooting.to_raw();
        actor.timer = 48;
        actor.sprite = 19;
        actor.set_travel_span(32);

        // Pause for 47 frames
        for i in 1..=47 {
            let ctx = FirePiranhaContext {
                frame: FrameTick(i),
                mario_travelling: false,
                mario_ref_x: 150,
                mario_ref_y: 150,
            };
            let eff = update(&mut actor, ctx);
            assert_eq!(eff, FirePiranhaEffect::Shooting);
            assert_eq!(actor.timer, (48 - i) as u8);
            assert_eq!(actor.sprite, 19);
        }

        // Frame 48: closes mouth (sprite 18), transitions to Retracting
        let ctx = FirePiranhaContext {
            frame: FrameTick(48),
            mario_travelling: false,
            mario_ref_x: 150,
            mario_ref_y: 150,
        };
        let eff = update(&mut actor, ctx);
        assert_eq!(eff, FirePiranhaEffect::Retracting);
        assert_eq!(actor.state, FirePiranhaState::Retracting.to_raw());
        assert_eq!(actor.sprite, 18);
        assert_eq!(actor.motion_leg, 32);

        // Retract 32 steps
        for i in 1..=32 {
            let ctx = FirePiranhaContext {
                frame: FrameTick(48 + i),
                mario_travelling: false,
                mario_ref_x: 150,
                mario_ref_y: 150,
            };
            update(&mut actor, ctx);
        }
        assert_eq!(actor.state, FirePiranhaState::Resting.to_raw());
        assert_eq!(actor.position.y, 200, "returned to exact starting position");
        assert_eq!(actor.timer, 4);

        // Resting: ticks down on frames where frame % 8 == 0
        // Frame 81..87: no tick
        for f in 81..=87 {
            let ctx = FirePiranhaContext {
                frame: FrameTick(f),
                mario_travelling: false,
                mario_ref_x: 150,
                mario_ref_y: 150,
            };
            let eff = update(&mut actor, ctx);
            assert_eq!(eff, FirePiranhaEffect::Resting);
            assert_eq!(actor.timer, 4);
        }
        // Frame 88 (88 % 8 == 0): timer becomes 3
        let ctx = FirePiranhaContext {
            frame: FrameTick(88),
            mario_travelling: false,
            mario_ref_x: 150,
            mario_ref_y: 150,
        };
        update(&mut actor, ctx);
        assert_eq!(actor.timer, 3);

        // Frames 96, 104, 112: timer becomes 2, 1, 0 -> transitions to Waiting!
        update(
            &mut actor,
            FirePiranhaContext {
                frame: FrameTick(96),
                mario_travelling: false,
                mario_ref_x: 150,
                mario_ref_y: 150,
            },
        );
        assert_eq!(actor.timer, 2);
        update(
            &mut actor,
            FirePiranhaContext {
                frame: FrameTick(104),
                mario_travelling: false,
                mario_ref_x: 150,
                mario_ref_y: 150,
            },
        );
        assert_eq!(actor.timer, 1);
        let eff = update(
            &mut actor,
            FirePiranhaContext {
                frame: FrameTick(112),
                mario_travelling: false,
                mario_ref_x: 150,
                mario_ref_y: 150,
            },
        );
        assert_eq!(eff, FirePiranhaEffect::Waiting);
        assert_eq!(actor.state, FirePiranhaState::Waiting.to_raw());
    }

    #[test]
    fn test_pipe_travel_despawn() {
        let mut actor = Actor::default();
        let ctx = FirePiranhaContext {
            frame: FrameTick(0),
            mario_travelling: true,
            mario_ref_x: 100,
            mario_ref_y: 100,
        };
        assert_eq!(update(&mut actor, ctx), FirePiranhaEffect::Despawn);
    }
}
