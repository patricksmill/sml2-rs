//! ROM `$5AD2D` (mode B anim 106): the rightward flashing mine.
//!
//! State 0 sets the horizontal flip, then uses `$5ACE4`'s timer. State 1
//! slides two pixels right and respawns once screen X reaches `$B8`.
use super::handler_0x5ace4::{self, Course, Effect, Error};
use crate::actor::{Actor, FrameTick};

pub(super) fn update(actor: &mut Actor, frame: FrameTick) -> Result<Effect, Error> {
    handler_0x5ace4::update(
        actor,
        frame,
        Course {
            dx: 2,
            screen_limit: 0xB8,
            face_right: true,
        },
    )
}
