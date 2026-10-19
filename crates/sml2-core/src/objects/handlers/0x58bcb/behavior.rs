//! ROM `$58BCB` (anim 40): the `$58BBA` vertical-platform family.
//!
//! Shares the `$58BDE` four-state table and every state body; only the
//! half-span immediate differs.
use super::handler_0x58bba::Kind;

/// ROM `$58BD7`: `ld a,20` (anim 40).
pub(super) const ANIM_40: Kind = Kind { span: 20 };
