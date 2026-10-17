//! ROM `$5AB4B` (mode B anim 99): the anim 98 shell with a 48-pixel left launch.
use super::handler_0x5aab9;

pub(super) use handler_0x5aab9::{Effect, Error};

pub(super) fn update(actor: &mut crate::actor::Actor, easy: bool) -> Result<Effect, Error> {
    handler_0x5aab9::update(actor, easy, -48)
}
