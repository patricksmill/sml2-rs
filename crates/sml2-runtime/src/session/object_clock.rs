//! Object time survives loaders: startup `$022A` clears HRAM once; `$EBBB`
//! clears `$AF00..$AFFF` (including `$AF08`) without touching `$FFF0`.
//! Ordinary updates toggle `$AF08` and increment `$FFF0` on phase 1
//! (`$8121..$812D`); the boss loop increments `$FFF0` every call (`$8DBA`).

use sml2_core::objects::ObjectSim;

use super::SessionState;

#[derive(Default)]
pub(super) struct ObjectClock(u8);

impl ObjectClock {
    /// Retain the engine's actual counter, including boss and demo updates.
    pub(super) fn remember(&mut self, state: &SessionState) {
        match state {
            SessionState::InLevel(ctx) => self.observe(&ctx.obj),
            SessionState::Demo(demo) => self.observe(&demo.ctx.obj),
            _ => {}
        }
    }

    pub(super) fn observe(&mut self, objects: &ObjectSim) {
        self.0 = objects.fff0;
    }

    /// A level loader clears scratch phase, but inherits the HRAM counter.
    /// Leave `ObjectSim::new`'s deliberate replay/fixture defaults alone.
    pub(super) fn initialize_level(&self, objects: &mut ObjectSim) {
        crate::game::initialize_level_objects(objects, self.0);
    }
}
