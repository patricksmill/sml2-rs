//! ROM `$5AEF4` (mode B anim 111): the shorter curved flyer.
//!
//! Same machine as `$5ADDD`, including its state-0 init. The legs use `$6F55`,
//! `$6F76`, `$6F97`, and `$6FB7`.
use super::handler_0x5addd::{self, Curves, Effect, Error};
use crate::actor::{Actor, FrameTick};

const H_CLIMB: &[u8] = &[
    4, 4, 3, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    128,
];
const H_DIVE: &[u8] = &[
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 4, 4,
    128,
];
const V_CLIMB: &[u8] = &[
    0, 1, 1, 1, 1, 1, 1, 1, 2, 1, 2, 1, 2, 1, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0,
    0, 0,
];
const V_DIVE: &[u8] = &[
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 1, 2, 1, 2, 1, 2, 1, 1, 1, 1, 1, 1,
    1, 0,
];

pub(super) fn update(
    actor: &mut Actor,
    frame: FrameTick,
    player_x: u16,
) -> Result<Effect, Error> {
    handler_0x5addd::update(
        actor,
        frame,
        player_x,
        Curves {
            climb_x: H_CLIMB,
            climb_y: V_CLIMB,
            dive_x: H_DIVE,
            dive_y: V_DIVE,
        },
    )
}
