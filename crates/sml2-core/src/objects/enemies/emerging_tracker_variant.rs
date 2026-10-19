//! ROM `$5934A` (anim 62): the `$5902D` emergence/tracking family.
//!
//! The entry gates on `$FFDF` and jumps into the same `$59032` init as
//! `$5902D`, then dispatches `$FFDF-1` through the `$59354` table
//! (`$5092`, `$50A8`, `$535C`, `$50D8`). States 1, 2 and 4 are byte-identical
//! to the anim-53 entry; only the state-3 wait body differs, so the body is
//! shared and this module carries the entry's own immediates.
use super::handler_0x5902d::StateThree;

/// ROM `$5936D`/`$59373`: `ld a,$70` -> `$FFDB`, `ld a,$30` -> `$FFEC` (anim 62).
pub(super) const STATE_THREE: StateThree = StateThree {
    sprite: 0x70,
    timer: 0x30,
};
