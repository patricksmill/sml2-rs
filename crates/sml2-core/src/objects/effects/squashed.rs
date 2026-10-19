//! Native squashed enemy countdown and removal behavior, ROM `$582A9`.
//!
//! Follows the idiomatic Rust porting guide: typed actor state, domain methods,
//! zero raw hex offsets or register plumbing.

use crate::actor::{Actor, ContactKind, FlashFlags};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SquashedState {
    Init = 0,
    CountingDown = 1,
}

impl SquashedState {
    pub const fn from_raw(value: u8) -> Self {
        match value {
            1 => Self::CountingDown,
            _ => Self::Init,
        }
    }

    pub const fn to_raw(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SquashedEffect {
    Initialized,
    CountingDown { remaining: u8 },
    Despawn,
}

/// Domain update for Squashed enemy (ROM $582A9).
pub fn update(actor: &mut Actor) -> SquashedEffect {
    let state = SquashedState::from_raw(actor.state);
    match state {
        SquashedState::Init => {
            actor.flash = FlashFlags::from_bits(16);
            actor.state = SquashedState::CountingDown.to_raw();
            actor.motion_phase = 32;
            actor.contact_kind = ContactKind::NONE;
            SquashedEffect::Initialized
        }
        SquashedState::CountingDown => {
            // $582C3: count motion_phase down; at 0 remove via $5B2D7.
            actor.motion_phase = actor.motion_phase.wrapping_sub(1);
            if actor.motion_phase == 0 {
                SquashedEffect::Despawn
            } else {
                SquashedEffect::CountingDown {
                    remaining: actor.motion_phase,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn squashed_init_configures_actor_countdown() {
        let mut actor = Actor::default();
        actor.contact_kind = ContactKind::from_raw(0x4E);

        let eff = update(&mut actor);
        assert_eq!(eff, SquashedEffect::Initialized);
        assert_eq!(actor.flash.bits(), 16);
        assert_eq!(actor.state, 1);
        assert_eq!(actor.motion_phase, 32);
        assert_eq!(actor.contact_kind, ContactKind::NONE);
    }

    #[test]
    fn squashed_countdown_decrements_and_despawns_at_zero() {
        let mut actor = Actor::default();
        actor.state = 1;
        actor.motion_phase = 2;

        let eff = update(&mut actor);
        assert_eq!(eff, SquashedEffect::CountingDown { remaining: 1 });
        assert_eq!(actor.motion_phase, 1);

        let eff = update(&mut actor);
        assert_eq!(eff, SquashedEffect::Despawn);
        assert_eq!(actor.motion_phase, 0);
    }
}
