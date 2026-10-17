//! ROM `$5945A` (anim 66): the `$59431` landing-walker family.
//!
//! Shares the `$582FB`/`$5831A` patrol body and the inline fall body. This
//! entry's landing contact is 72 and it refreshes flash 16 on every fall frame.
use super::handler_0x59431::Kind;

/// ROM `$59482`: contact 72; `$59467` sets flash 16 before the ground probe.
pub(super) const ANIM_66: Kind = Kind {
    contact: 72,
    flash_on_fall: true,
};
