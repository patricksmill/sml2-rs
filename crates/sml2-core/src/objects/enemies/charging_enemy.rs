//! Native charging enemy domain behavior, ROM `$5A9E8` (anim 83 Mode B).
//!
//! Spawns with defeat_mode 0x80 (invulnerable/armored).
//! Waits in State 5 until Mario is at least 40 pixels away horizontally.
//! Then turns toward Mario and enters State 1 (left) or State 2 (right).
//! In State 1/2: pauses for 24 frames, then creeps forward for 24 steps at 1 px/frame.
//! When 24 steps finish: becomes vulnerable (defeat_mode 0), plays SFX 16,
//! and enters State 3 (charge left) or State 4 (charge right) at 3 px/frame.
//! Resets at the native screen-X gate ($B8) or beyond the enlarged view's
//! world bounds, returning to its spawn point ($5B2BF).
//!
//! Follows the idiomatic Rust porting guide: typed actor state, domain methods,
//! zero raw hex offsets or register plumbing.

use crate::actor::{Actor, ContactKind, SpriteFlipFlags};

pub const DISTANCE_THRESHOLD_X: u8 = 40; // ROM $5AA0D: cp $28 (40)
pub const INITIAL_CREEP_BUDGET: u8 = 24; // ROM $5A9FD: ld a, 24
pub const INITIAL_PAUSE_FRAMES: u8 = 24; // ROM $5AA01: ld a, 24
pub const CONTACT_KIND: u8 = 74; // ROM $5A9F9: ld a, 74 (0x4A)
pub const CHARGE_SFX: u8 = 16; // ROM $5AA4D: ld a, 16 (sSfxRequest1)
pub const OFFSCREEN_SCREEN_X: u8 = 184; // ROM $5AA61: cp $B8 (184)

/// Charging enemy movement state ($5A9E8 / $6A20 / $6A30 / $6A53 / $6A5A / $6A0A: states 0..=5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChargingEnemyState {
    Init = 0,
    CreepLeft = 1,
    CreepRight = 2,
    ChargeLeft = 3,
    ChargeRight = 4,
    Wait = 5,
}

impl ChargingEnemyState {
    pub const fn from_raw(value: u8) -> Self {
        match value {
            1 => Self::CreepLeft,
            2 => Self::CreepRight,
            3 => Self::ChargeLeft,
            4 => Self::ChargeRight,
            5 => Self::Wait,
            _ => Self::Init,
        }
    }

    pub const fn to_raw(self) -> u8 {
        self as u8
    }
}

#[derive(Default)]
pub struct ChargingEnemyContext {
    pub mario_ref_x: u16,
    /// World bounds for enlarged views. Native play keeps the ROM's wrapped
    /// distance and preceding draw's screen-X reset gate.
    pub charge_bounds: Option<std::ops::Range<i32>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChargingEnemyEffect {
    None,
    Initialized,
    FacingSelected { facing: u8 },
    Creeping { state: ChargingEnemyState },
    ChargeTriggered { sfx: u8, state: ChargingEnemyState },
    Charging { state: ChargingEnemyState },
    OffscreenReset,
}

/// Domain update for ChargingEnemy (ROM $5A9E8).
pub fn update(actor: &mut Actor, context: ChargingEnemyContext) -> ChargingEnemyEffect {
    let state = ChargingEnemyState::from_raw(actor.state);
    match state {
        ChargingEnemyState::Init => {
            actor.position.move_y(2);
            actor.defeat_mode = 0x80;
            actor.sprite_flip = SpriteFlipFlags::default();
            actor.contact_kind = ContactKind::from_raw(CONTACT_KIND);
            actor.set_travel_span(INITIAL_CREEP_BUDGET);
            actor.timer = INITIAL_PAUSE_FRAMES;
            actor.state = ChargingEnemyState::Wait.to_raw();
            ChargingEnemyEffect::Initialized
        }
        ChargingEnemyState::Wait => {
            // ROM $5AA0A: abs(sMarioRefXLo - sObjectXLo)
            let mario_lo = context.mario_ref_x as u8;
            let actor_lo = actor.position.x as u8;
            let diff = mario_lo.wrapping_sub(actor_lo);
            let dist = if context.charge_bounds.is_some() {
                context.mario_ref_x.abs_diff(actor.position.x)
            } else {
                u16::from(if diff & 0x80 == 0 {
                    diff
                } else {
                    diff.wrapping_neg()
                })
            };

            // ROM $5AA0D: cp $28 (ret c if < 40)
            if dist < u16::from(DISTANCE_THRESHOLD_X) {
                return ChargingEnemyEffect::None;
            }

            // ROM $5AA10: call 5B3C6 (face toward Mario)
            let facing = match actor.face_toward(context.mario_ref_x) {
                std::cmp::Ordering::Greater => 1,
                std::cmp::Ordering::Less => 0xFF,
                std::cmp::Ordering::Equal => 0,
            };
            ChargingEnemyEffect::FacingSelected { facing }
        }
        ChargingEnemyState::CreepLeft => {
            if actor.timer != 0 {
                actor.timer = actor.timer.wrapping_sub(1);
                ChargingEnemyEffect::None
            } else {
                actor.position.move_x(-1);
                let budget = actor.travel_span().wrapping_sub(1);
                actor.set_travel_span(budget);
                if budget != 0 {
                    ChargingEnemyEffect::Creeping {
                        state: ChargingEnemyState::CreepLeft,
                    }
                } else {
                    actor.defeat_mode = 0;
                    actor.state = ChargingEnemyState::ChargeLeft.to_raw();
                    ChargingEnemyEffect::ChargeTriggered {
                        sfx: CHARGE_SFX,
                        state: ChargingEnemyState::ChargeLeft,
                    }
                }
            }
        }
        ChargingEnemyState::CreepRight => {
            if actor.timer != 0 {
                actor.timer = actor.timer.wrapping_sub(1);
                ChargingEnemyEffect::None
            } else {
                actor.position.move_x(1);
                let budget = actor.travel_span().wrapping_sub(1);
                actor.set_travel_span(budget);
                if budget != 0 {
                    ChargingEnemyEffect::Creeping {
                        state: ChargingEnemyState::CreepRight,
                    }
                } else {
                    actor.defeat_mode = 0;
                    actor.state = ChargingEnemyState::ChargeRight.to_raw();
                    ChargingEnemyEffect::ChargeTriggered {
                        sfx: CHARGE_SFX,
                        state: ChargingEnemyState::ChargeRight,
                    }
                }
            }
        }
        ChargingEnemyState::ChargeLeft | ChargingEnemyState::ChargeRight => {
            let offscreen = context
                .charge_bounds
                .as_ref()
                .map_or(actor.screen.x >= OFFSCREEN_SCREEN_X, |bounds| {
                    !bounds.contains(&i32::from(actor.position.x))
                });
            actor
                .position
                .move_x(if state == ChargingEnemyState::ChargeLeft {
                    -3
                } else {
                    3
                });
            if offscreen {
                ChargingEnemyEffect::OffscreenReset
            } else {
                ChargingEnemyEffect::Charging { state }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charging_enemy_init_sets_defeat_mode_and_wait_state() {
        let mut actor = Actor::default();
        actor.position.y = 100;

        let eff = update(
            &mut actor,
            ChargingEnemyContext {
                mario_ref_x: 0,
                ..Default::default()
            },
        );
        assert_eq!(eff, ChargingEnemyEffect::Initialized);
        assert_eq!(actor.position.y, 102, "moves Y by +2");
        assert_eq!(actor.defeat_mode, 0x80, "defeat mode armored 0x80");
        assert_eq!(actor.contact_kind, ContactKind::from_raw(CONTACT_KIND));
        assert_eq!(actor.travel_span(), 24);
        assert_eq!(actor.timer, 24);
        assert_eq!(actor.state, ChargingEnemyState::Wait.to_raw());
    }

    #[test]
    fn charging_enemy_waits_when_mario_is_closer_than_40_px() {
        let mut actor = Actor::default();
        actor.position.x = 100;
        actor.state = ChargingEnemyState::Wait.to_raw();

        // Mario 20 px away (< 40)
        let eff = update(
            &mut actor,
            ChargingEnemyContext {
                mario_ref_x: 120,
                ..Default::default()
            },
        );
        assert_eq!(eff, ChargingEnemyEffect::None);
        assert_eq!(actor.state, ChargingEnemyState::Wait.to_raw());
    }

    #[test]
    fn charging_enemy_triggers_when_mario_is_40_px_away() {
        let mut actor = Actor::default();
        actor.position.x = 100;
        actor.state = ChargingEnemyState::Wait.to_raw();

        // Mario 50 px to the right (>= 40)
        let eff = update(
            &mut actor,
            ChargingEnemyContext {
                mario_ref_x: 150,
                ..Default::default()
            },
        );
        assert_eq!(eff, ChargingEnemyEffect::FacingSelected { facing: 1 });
        assert_eq!(actor.state, ChargingEnemyState::CreepRight.to_raw());
        assert!(actor.sprite_flip.horizontal());
    }

    #[test]
    fn charging_enemy_creeps_then_charges_and_resets_offscreen() {
        let mut actor = Actor::default();
        actor.position.x = 100;
        actor.state = ChargingEnemyState::CreepLeft.to_raw();
        actor.timer = 24;
        actor.set_travel_span(24);
        actor.defeat_mode = 0x80;
        actor.screen.x = 80;

        // 24 frames of pause
        for t in 1..=24 {
            let eff = update(
                &mut actor,
                ChargingEnemyContext {
                    mario_ref_x: 0,
                    ..Default::default()
                },
            );
            assert_eq!(eff, ChargingEnemyEffect::None);
            assert_eq!(actor.timer, 24 - t as u8);
            assert_eq!(actor.position.x, 100);
        }

        // 23 creep steps
        for step in 1..=23 {
            let eff = update(
                &mut actor,
                ChargingEnemyContext {
                    mario_ref_x: 0,
                    ..Default::default()
                },
            );
            assert_eq!(
                eff,
                ChargingEnemyEffect::Creeping {
                    state: ChargingEnemyState::CreepLeft
                }
            );
            assert_eq!(actor.position.x, 100 - step);
            assert_eq!(actor.travel_span(), 24 - step as u8);
        }

        // 24th creep step triggers charge!
        let eff = update(
            &mut actor,
            ChargingEnemyContext {
                mario_ref_x: 0,
                ..Default::default()
            },
        );
        assert_eq!(
            eff,
            ChargingEnemyEffect::ChargeTriggered {
                sfx: CHARGE_SFX,
                state: ChargingEnemyState::ChargeLeft,
            }
        );
        assert_eq!(
            actor.defeat_mode, 0,
            "defeat mode cleared to 0 (vulnerable)"
        );
        assert_eq!(actor.state, ChargingEnemyState::ChargeLeft.to_raw());

        // Charge step 1: moves X by -3
        let eff = update(
            &mut actor,
            ChargingEnemyContext {
                mario_ref_x: 0,
                ..Default::default()
            },
        );
        assert_eq!(
            eff,
            ChargingEnemyEffect::Charging {
                state: ChargingEnemyState::ChargeLeft
            }
        );
        assert_eq!(actor.position.x, 100 - 24 - 3);

        // When offscreen (screen.x >= 184 / 0xB8): returns OffscreenReset
        actor.screen.x = 184;
        let eff = update(
            &mut actor,
            ChargingEnemyContext {
                mario_ref_x: 0,
                ..Default::default()
            },
        );
        assert_eq!(eff, ChargingEnemyEffect::OffscreenReset);
    }
}
