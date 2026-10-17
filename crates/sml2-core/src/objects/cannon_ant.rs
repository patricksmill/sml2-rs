//! Native cannon ant (Dokanto) enemy domain behavior, ROM `$5A773` (anim 77 Mode B).
//!
//! Walks horizontally (initially 24 steps, then 48 steps), then pauses to aim its cannon.
//! If Mario is above, it aims diagonally upward (sprite 148); otherwise horizontally (sprite 147).
//! After a 16-frame aiming pause, it fires a projectile (anim 78, sprite 145, SFX 17).
//! After a 32-frame recovery pause, it resets to walking sprite 146, turns around,
//! reloads its 48-step budget, and walks in the opposite direction.
//!
//! Follows the idiomatic Rust porting guide: typed actor state, domain methods,
//! zero raw hex offsets or register plumbing.

use crate::actor::{Actor, ContactKind, Facing, FrameTick};

pub const INITIAL_WALK_BUDGET: u8 = 24; // ROM $5A77F: ld a, 24
pub const FULL_WALK_BUDGET: u8 = 48;    // ROM $5A783: add a (48)
pub const AIM_PAUSE_FRAMES: u8 = 16;    // ROM $5A7B3: ld a, 16
pub const RECOVERY_FRAMES: u8 = 32;     // ROM $5A7F7: ld a, 32
pub const CONTACT_KIND: u8 = 77;        // ROM $5A778: ld a, 77 (0x4D)

pub const SPRITE_PROJECTILE: u8 = 145; // ROM $5A7D4: ld a, 145
pub const SPRITE_WALK: u8 = 146;       // ROM $5A807: ld a, 146
pub const SPRITE_AIM_HORIZ: u8 = 147;  // ROM $5A7C1: ld a, 147
pub const SPRITE_AIM_UP: u8 = 148;     // ROM $5A7C6: ld a, 148

pub const PROJECTILE_ANIM: u8 = 78;    // ROM $5A7EB: ld a, 78
pub const FIRE_SFX: u8 = 17;           // ROM $5A7FB: ld a, 17 (sSfxRequest1)

/// Cannon ant movement state ($5A773 / $6793 / $679D / $67CB / $6801: states 0..=4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CannonAntState {
    Init = 0,
    WalkLeft = 1,
    WalkRight = 2,
    Aim = 3,
    Recovery = 4,
}

impl CannonAntState {
    pub const fn from_raw(value: u8) -> Self {
        match value {
            1 => Self::WalkLeft,
            2 => Self::WalkRight,
            3 => Self::Aim,
            4 => Self::Recovery,
            _ => Self::Init,
        }
    }

    pub const fn to_raw(self) -> u8 {
        self as u8
    }
}

pub struct CannonAntContext {
    pub frame: FrameTick,
    pub mario_ref_x: u16,
    pub mario_ref_y: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CannonAntEffect {
    None,
    Initialized { facing: u8 },
    Walking { state: CannonAntState },
    Aiming { sprite: u8 },
    FireProjectile { anim: u8, sprite: u8, attr: u8, sfx: u8 },
    Turned { state: CannonAntState },
}

/// Domain update for CannonAnt (ROM $5A773).
pub fn update(
    actor: &mut Actor,
    context: CannonAntContext,
) -> CannonAntEffect {
    let state = CannonAntState::from_raw(actor.state);
    match state {
        CannonAntState::Init => {
            actor.contact_kind = ContactKind::from_raw(CONTACT_KIND);
            let (next_state, flip, facing) = match context.mario_ref_x.cmp(&actor.position.x) {
                std::cmp::Ordering::Greater => (CannonAntState::WalkRight.to_raw(), true, 1),
                std::cmp::Ordering::Less => (CannonAntState::WalkLeft.to_raw(), false, 0xFF),
                std::cmp::Ordering::Equal => (CannonAntState::WalkLeft.to_raw(), false, 0),
            };
            actor.state = next_state;
            actor.sprite_flip.set_horizontal(flip);
            actor.travel_direction = Facing::from_raw(INITIAL_WALK_BUDGET);
            actor.motion_leg = FULL_WALK_BUDGET;
            CannonAntEffect::Initialized { facing }
        }
        CannonAntState::WalkLeft => {
            actor.animate_two_frame_toggle(context.frame, 4);
            actor.position.move_x(-1);
            let budget = actor.travel_direction.raw().wrapping_sub(1);
            actor.travel_direction = Facing::from_raw(budget);
            if budget != 0 {
                CannonAntEffect::Walking {
                    state: CannonAntState::WalkLeft,
                }
            } else {
                actor.visibility_mask = CannonAntState::WalkLeft.to_raw();
                transition_to_aim(actor, context.mario_ref_y)
            }
        }
        CannonAntState::WalkRight => {
            actor.animate_two_frame_toggle(context.frame, 4);
            actor.position.move_x(1);
            let budget = actor.travel_direction.raw().wrapping_sub(1);
            actor.travel_direction = Facing::from_raw(budget);
            if budget != 0 {
                CannonAntEffect::Walking {
                    state: CannonAntState::WalkRight,
                }
            } else {
                actor.visibility_mask = CannonAntState::WalkRight.to_raw();
                transition_to_aim(actor, context.mario_ref_y)
            }
        }
        CannonAntState::Aim => {
            if actor.timer != 0 {
                actor.timer = actor.timer.wrapping_sub(1);
                CannonAntEffect::None
            } else {
                // ROM $5A7D4..$5A7EB: spawn projectile
                let attr = actor.sprite_flip.bits() | if actor.sprite == SPRITE_AIM_UP { 1 } else { 0 };
                actor.state = CannonAntState::Recovery.to_raw();
                actor.timer = RECOVERY_FRAMES;
                CannonAntEffect::FireProjectile {
                    anim: PROJECTILE_ANIM,
                    sprite: SPRITE_PROJECTILE,
                    attr,
                    sfx: FIRE_SFX,
                }
            }
        }
        CannonAntState::Recovery => {
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer != 0 {
                CannonAntEffect::None
            } else {
                // ROM $5A807..$5A813: restore walk sprite, reverse direction and reload budget
                actor.sprite = SPRITE_WALK;
                let saved_walk_state = actor.visibility_mask;
                actor.state = saved_walk_state ^ 3;
                actor.sprite_flip.toggle_horizontal();
                actor.travel_direction = Facing::from_raw(actor.motion_leg);
                CannonAntEffect::Turned {
                    state: CannonAntState::from_raw(actor.state),
                }
            }
        }
    }
}

fn transition_to_aim(actor: &mut Actor, mario_y: u16) -> CannonAntEffect {
    actor.state = CannonAntState::Aim.to_raw();
    actor.timer = AIM_PAUSE_FRAMES;
    // ROM $5B41A: check if Mario is above object
    let aim_sprite = if mario_y < actor.position.y {
        SPRITE_AIM_UP
    } else {
        SPRITE_AIM_HORIZ
    };
    actor.sprite = aim_sprite;
    CannonAntEffect::Aiming { sprite: aim_sprite }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cannon_ant_init_faces_mario_and_sets_initial_budgets() {
        let mut actor = Actor::default();
        actor.position.x = 100;

        // Mario on right
        let eff = update(
            &mut actor,
            CannonAntContext {
                frame: FrameTick(0),
                mario_ref_x: 150,
                mario_ref_y: 100,
            },
        );
        assert_eq!(eff, CannonAntEffect::Initialized { facing: 1 });
        assert_eq!(actor.state, CannonAntState::WalkRight.to_raw());
        assert!(actor.sprite_flip.horizontal());
        assert_eq!(actor.contact_kind, ContactKind::from_raw(CONTACT_KIND));
        assert_eq!(actor.travel_direction.raw(), 24);
        assert_eq!(actor.motion_leg, 48);
    }

    #[test]
    fn cannon_ant_walks_24_steps_then_aims_up_at_mario() {
        let mut actor = Actor::default();
        actor.position.x = 100;
        actor.position.y = 80;

        // Init: Mario to left, above
        update(
            &mut actor,
            CannonAntContext {
                frame: FrameTick(1),
                mario_ref_x: 50,
                mario_ref_y: 40,
            },
        );
        assert_eq!(actor.state, CannonAntState::WalkLeft.to_raw());
        assert!(!actor.sprite_flip.horizontal());

        // Walk 23 steps
        for step in 1..=23 {
            let eff = update(
                &mut actor,
                CannonAntContext {
                    frame: FrameTick(1),
                    mario_ref_x: 50,
                    mario_ref_y: 40,
                },
            );
            assert_eq!(
                eff,
                CannonAntEffect::Walking {
                    state: CannonAntState::WalkLeft
                }
            );
            assert_eq!(actor.position.x, 100 - step);
            assert_eq!(actor.travel_direction.raw(), 24 - step as u8);
        }

        // 24th step reaches 0: transitions to Aim, aims up at Mario (sprite 148)
        let eff = update(
            &mut actor,
            CannonAntContext {
                frame: FrameTick(1),
                mario_ref_x: 50,
                mario_ref_y: 40,
            },
        );
        assert_eq!(eff, CannonAntEffect::Aiming { sprite: SPRITE_AIM_UP });
        assert_eq!(actor.state, CannonAntState::Aim.to_raw());
        assert_eq!(actor.sprite, SPRITE_AIM_UP);
        assert_eq!(actor.timer, 16);
        assert_eq!(actor.visibility_mask, 1, "saved walk state");
    }

    #[test]
    fn cannon_ant_aims_horizontally_when_mario_is_below() {
        let mut actor = Actor::default();
        actor.position.x = 100;
        actor.position.y = 80;
        actor.state = CannonAntState::WalkLeft.to_raw();
        actor.travel_direction = Facing::from_raw(1);

        // Step 1: budget reaches 0, Mario below at Y = 100
        let eff = update(
            &mut actor,
            CannonAntContext {
                frame: FrameTick(1),
                mario_ref_x: 50,
                mario_ref_y: 100,
            },
        );
        assert_eq!(eff, CannonAntEffect::Aiming { sprite: SPRITE_AIM_HORIZ });
        assert_eq!(actor.sprite, SPRITE_AIM_HORIZ);
        assert_eq!(actor.timer, 16);
    }

    #[test]
    fn cannon_ant_fires_after_16_frames_and_recovers_to_opposite_direction() {
        let mut actor = Actor::default();
        actor.position.x = 80;
        actor.position.y = 80;
        actor.state = CannonAntState::Aim.to_raw();
        actor.timer = 16;
        actor.sprite = SPRITE_AIM_UP;
        actor.visibility_mask = 1; // was walking left
        actor.motion_leg = 48;

        // 16 frames of aiming countdown
        for t in 1..=16 {
            let eff = update(
                &mut actor,
                CannonAntContext {
                    frame: FrameTick(1),
                    mario_ref_x: 50,
                    mario_ref_y: 40,
                },
            );
            assert_eq!(eff, CannonAntEffect::None);
            assert_eq!(actor.timer, 16 - t as u8);
        }

        // Timer is now 0: next frame fires projectile
        let eff = update(
            &mut actor,
            CannonAntContext {
                frame: FrameTick(1),
                mario_ref_x: 50,
                mario_ref_y: 40,
            },
        );
        assert_eq!(
            eff,
            CannonAntEffect::FireProjectile {
                anim: PROJECTILE_ANIM,
                sprite: SPRITE_PROJECTILE,
                attr: 1, // bit 0 set for upward aim, no horizontal flip
                sfx: FIRE_SFX,
            }
        );
        assert_eq!(actor.state, CannonAntState::Recovery.to_raw());
        assert_eq!(actor.timer, 32);

        // 31 frames of recovery countdown
        for t in 1..=31 {
            let eff = update(
                &mut actor,
                CannonAntContext {
                    frame: FrameTick(1),
                    mario_ref_x: 50,
                    mario_ref_y: 40,
                },
            );
            assert_eq!(eff, CannonAntEffect::None);
            assert_eq!(actor.timer, 32 - t as u8);
        }

        // 32nd recovery frame: timer reaches 0, resets to walking sprite 146,
        // reverses direction to WalkRight (2), toggles horizontal flip, sets budget 48
        let eff = update(
            &mut actor,
            CannonAntContext {
                frame: FrameTick(1),
                mario_ref_x: 50,
                mario_ref_y: 40,
            },
        );
        assert_eq!(eff, CannonAntEffect::Turned { state: CannonAntState::WalkRight });
        assert_eq!(actor.state, CannonAntState::WalkRight.to_raw());
        assert_eq!(actor.sprite, SPRITE_WALK);
        assert!(actor.sprite_flip.horizontal());
        assert_eq!(actor.travel_direction.raw(), 48);
    }
}
