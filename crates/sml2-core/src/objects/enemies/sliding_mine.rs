//! ROM `$5AD2D` (mode B anim 106): the rightward flashing mine.
//!
//! State 0 sets the horizontal flip, then uses `$5ACE4`'s timer. State 1
//! slides two pixels right and respawns once screen X reaches `$B8`.
use super::handler_0x5ace4::{self, Course, Effect, Error};
use crate::actor::{Actor, FrameTick};

pub(super) const SCREEN_LIMIT: u8 = 0xB8;

/// `view_bounds`: see [`Course::view_bounds`].
pub(super) fn update(
    actor: &mut Actor,
    frame: FrameTick,
    view_bounds: Option<std::ops::Range<i32>>,
) -> Result<Effect, Error> {
    handler_0x5ace4::update(
        actor,
        frame,
        Course {
            dx: 2,
            screen_limit: SCREEN_LIMIT,
            face_right: true,
            view_bounds,
        },
    )
}
