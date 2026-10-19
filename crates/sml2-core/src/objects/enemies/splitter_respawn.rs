//! ROM `$5AA68` (mode B anim 84): the anim-54 respawn tail without the money bag.
use super::handler_0x5917a::{self, Effect, Error};
use crate::actor::Actor;

pub(super) fn update(actor: &mut Actor) -> Result<Effect, Error> {
    if actor.state == 0 {
        actor.begin_defeat_hop();
        Ok(Effect::Continue)
    } else {
        handler_0x5917a::update(actor)
    }
}
