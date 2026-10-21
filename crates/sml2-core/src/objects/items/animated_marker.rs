//! Handler ROM `0x586EB` (bank `$16`, CPU `$46EB`) — anim 30 in both profile tables.
//!
//! `$5B6EB`: `call $7684; ldh a,[$FFD5]; add $66; ldh [$FFE2],a; ret`.
//! `$7684` is the 4-frame flash toggle (`$FFDE ^= $10` when `$FFF0 & 3 == 0`),
//! not the every-other-frame flash of the other items.
use super::item_poses;
use crate::actor::{Actor, FrameTick};

pub(super) fn update(actor: &mut Actor, frame: FrameTick) {
    actor.flash.animate_palette(frame);
    item_poses::update_marker66(actor);
}
