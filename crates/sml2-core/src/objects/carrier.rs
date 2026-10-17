//! Native carrier platform domain behavior, ROM `$58DD1` (anim 46).
//!
//! Follows the idiomatic Rust porting guide: typed actor state, domain methods,
//! zero raw hex offsets or register plumbing.

use crate::actor::{Actor, ContactKind};

/// Carrier movement state ($4DD1 / $4DE5 / $4E04: states 1..=2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarrierState {
    Init = 0,
    Falling = 1,
    Waiting = 2,
}

impl CarrierState {
    pub const fn from_raw(value: u8) -> Self {
        match value {
            1 => Self::Falling,
            2 => Self::Waiting,
            _ => Self::Init,
        }
    }

    pub const fn to_raw(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarrierEffect {
    Initialized,
    Waiting,
    Activated,
    TimerTicking,
    Falling { carry_dy: u8 },
    Despawn { carry_dy: u8 },
}

pub trait CarrierWorld {
    fn step_fall(&mut self, actor: &mut Actor) -> u8;
    fn probe_ground(&mut self, actor: &Actor) -> bool;
}

/// Domain update for Carrier platform (ROM $58DD1, anim 46).
pub fn update<W>(
    actor: &mut Actor,
    world: &mut W,
) -> CarrierEffect
where
    W: CarrierWorld,
{
    let state = CarrierState::from_raw(actor.state);
    match state {
        CarrierState::Init => {
            // $4DD1: the one-shot init. Type $20 (on-object support), wait state 2.
            actor.contact_kind = ContactKind::from_raw(0x20);
            actor.state = CarrierState::Waiting.to_raw();
            CarrierEffect::Initialized
        }
        CarrierState::Waiting => {
            // $4E04: wait until Mario stands on the platform (interactions bit 3).
            if !actor.interactions.stood_on() {
                return CarrierEffect::Waiting;
            }
            actor.state = CarrierState::Falling.to_raw();
            actor.timer = 0x1C;
            CarrierEffect::Activated
        }
        CarrierState::Falling => {
            // $4DE5: countdown, then move chain.
            if actor.timer != 0 {
                actor.timer = actor.timer.wrapping_sub(1);
                if actor.timer != 0 {
                    return CarrierEffect::TimerTicking;
                }
            }
            let dy = world.step_fall(actor);
            actor.position.move_y(i16::from(dy));
            if world.probe_ground(actor) {
                CarrierEffect::Despawn { carry_dy: dy }
            } else {
                CarrierEffect::Falling { carry_dy: dy }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::InteractionFlags;

    struct MockWorld {
        fall_dy: u8,
        ground: bool,
    }

    impl CarrierWorld for MockWorld {
        fn step_fall(&mut self, _actor: &mut Actor) -> u8 {
            self.fall_dy
        }
        fn probe_ground(&mut self, _actor: &Actor) -> bool {
            self.ground
        }
    }

    #[test]
    fn carrier_init_configures_contact_kind_and_waiting_state() {
        let mut world = MockWorld { fall_dy: 0, ground: false };
        let mut actor = Actor::default();

        let eff = update(&mut actor, &mut world);
        assert_eq!(eff, CarrierEffect::Initialized);
        assert_eq!(actor.contact_kind.raw(), 0x20);
        assert_eq!(actor.state, 2);
    }

    #[test]
    fn carrier_waiting_transitions_on_stood_on_interaction() {
        let mut world = MockWorld { fall_dy: 0, ground: false };
        let mut actor = Actor::default();
        actor.state = 2;

        // Not stood on -> remains waiting
        let eff = update(&mut actor, &mut world);
        assert_eq!(eff, CarrierEffect::Waiting);
        assert_eq!(actor.state, 2);

        // Stood on -> transitions to falling with 28-frame delay
        actor.interactions = InteractionFlags::from_bits(0x08);
        let eff = update(&mut actor, &mut world);
        assert_eq!(eff, CarrierEffect::Activated);
        assert_eq!(actor.state, 1);
        assert_eq!(actor.timer, 28);
    }

    #[test]
    fn carrier_falling_counts_down_then_steps_and_despawns_on_ground() {
        let mut actor = Actor::default();
        actor.state = 1;
        actor.timer = 2;
        actor.position.y = 100;

        let mut world = MockWorld { fall_dy: 3, ground: false };

        // Tick 1: timer 2 -> 1, no motion
        let eff = update(&mut actor, &mut world);
        assert_eq!(eff, CarrierEffect::TimerTicking);
        assert_eq!(actor.timer, 1);
        assert_eq!(actor.position.y, 100);

        // Tick 2: timer 1 -> 0: steps fall immediately, moves Y by 3, airborne
        let eff = update(&mut actor, &mut world);
        assert_eq!(eff, CarrierEffect::Falling { carry_dy: 3 });
        assert_eq!(actor.timer, 0);
        assert_eq!(actor.position.y, 103);

        // Tick 3: ground reached -> despawns
        world.ground = true;
        let eff = update(&mut actor, &mut world);
        assert_eq!(eff, CarrierEffect::Despawn { carry_dy: 3 });
        assert_eq!(actor.position.y, 106);
    }
}
