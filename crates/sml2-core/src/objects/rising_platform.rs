//! Rising platform, ROM $58E12: rider-controlled ascent and delayed return.
use crate::actor::{Actor, ContactKind};

const SPAN: u8 = 64;
const RETURN_DELAY: u8 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RisingPlatformState {
    Init = 0,
    Rising = 1,
    Falling = 2,
    Waiting = 3,
}

impl RisingPlatformState {
    pub const fn to_raw(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RisingPlatformEffect {
    Initialized,
    Waiting,
    Activated,
    Rising { carry_dy: Option<i16> },
    ReachedApex,
    SteppedOff,
    SteppedOn,
    Delaying,
    Descending,
    RestingAtBottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownState(pub u8);

impl TryFrom<u8> for RisingPlatformState {
    type Error = UnknownState;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Init),
            1 => Ok(Self::Rising),
            2 => Ok(Self::Falling),
            3 => Ok(Self::Waiting),
            state => Err(UnknownState(state)),
        }
    }
}

pub fn update(actor: &mut Actor) -> Result<RisingPlatformEffect, UnknownState> {
    use RisingPlatformEffect::*;
    let riding = actor.interactions.stood_on();
    Ok(match RisingPlatformState::try_from(actor.state)? {
        RisingPlatformState::Init => {
            actor.contact_kind = ContactKind::from_raw(32);
            actor.state = RisingPlatformState::Waiting.to_raw();
            Initialized
        }
        RisingPlatformState::Waiting if riding => {
            actor.state = RisingPlatformState::Rising.to_raw();
            actor.motion_leg = SPAN;
            actor.set_travel_span(SPAN);
            Activated
        }
        RisingPlatformState::Waiting => Waiting,
        RisingPlatformState::Rising if !riding => {
            actor.state = RisingPlatformState::Falling.to_raw();
            actor.timer = RETURN_DELAY;
            SteppedOff
        }
        RisingPlatformState::Rising if actor.motion_leg == 0 => ReachedApex,
        RisingPlatformState::Rising => {
            actor.motion_leg -= 1;
            actor.position.move_y(-1);
            Rising { carry_dy: Some(-1) }
        }
        RisingPlatformState::Falling if riding => {
            actor.state = RisingPlatformState::Rising.to_raw();
            actor.timer = 0;
            SteppedOn
        }
        RisingPlatformState::Falling => {
            if actor.timer != 0 {
                actor.timer -= 1;
                if actor.timer != 0 {
                    return Ok(Delaying);
                }
            }
            if actor.motion_leg == actor.travel_span() {
                RestingAtBottom
            } else {
                actor.motion_leg = actor.motion_leg.wrapping_add(1);
                actor.position.move_y(1);
                Descending
            }
        }
    })
}

#[cfg(test)]
#[path = "rising_platform_tests.rs"]
mod tests;
