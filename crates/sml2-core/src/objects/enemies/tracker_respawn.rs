//! ROM `$5917A` (mode A anim 54): a defeated enemy that rebounds, then falls
//! back into its spawn record as the respawning anim 53.
//!
//! State 0 arms flash 16, counts the money bag, and starts the `$581B9` hop.
//! State 1 runs the `$581CE` rise on the eight-step `$74AB` curve. State 2
//! falls on the `$5B451` curve; once screen Y reaches `$A0` the definition
//! drops 54 -> 53 and `$5B2BF` restores the spawn record.
use super::falling_dropper::step_fall_speed;
use std::ops::Range;

use crate::actor::{Actor, FlashFlags, MotionProfileError};

const FLASH: u8 = 16;
pub(super) const FALL_LIMIT: u8 = 0xA0;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
    Rise(MotionProfileError),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Continue,
    MoneyBag,
    Respawn,
}

/// `view_bounds`: an enlarged view's world-Y band for the fall limit, from
/// `ObjectSim::vertical_reset_bounds`; `None` keeps the native byte test.
pub(super) fn update(actor: &mut Actor, view_bounds: Option<Range<i32>>) -> Result<Effect, Error> {
    match actor.state {
        // $5917F: flash 16, $5B31D money bag, then the shared $581B9 hop.
        0 => {
            actor.flash = FlashFlags::from_bits(FLASH);
            actor.begin_defeat_hop();
            Ok(Effect::MoneyBag)
        }
        // $59189 dec -> rst $28[0] = $581CE: rise on 2x the $74AB curve.
        1 => {
            actor.advance_defeat_rise().map_err(Error::Rise)?;
            Ok(Effect::Continue)
        }
        // $59189 dec -> rst $28[1] = $5918F: fall on the $5B451 curve.
        2 => {
            let offscreen = view_bounds.map_or(actor.screen.y >= FALL_LIMIT, |b| {
                !b.contains(&i32::from(actor.position.y))
            });
            let speed = step_fall_speed(actor);
            actor.position.move_y(i16::from(speed));
            if !offscreen {
                return Ok(Effect::Continue);
            }
            actor.definition = actor.definition.wrapping_sub(1);
            Ok(Effect::Respawn)
        }
        // The source table has two targets; anything else reads past it.
        state => Err(Error::InvalidState(state)),
    }
}
