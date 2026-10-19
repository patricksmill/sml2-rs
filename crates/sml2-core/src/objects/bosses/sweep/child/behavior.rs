//! Horizontal projectile, boss anim 17, ROM `$A27E` (bank 2).
use crate::actor::{Actor, ContactKind, FrameTick};

/// Return the optional sound request; unsupported state bytes stay diagnostic.
pub(super) fn update(actor: &mut Actor, frame: FrameTick) -> Result<Option<u8>, u8> {
    match actor.state {
        0 => {
            if actor.sprite == 14 {
                return Ok(None);
            }
            actor.contact_kind = ContactKind::from_raw(64);
            // Home-bank `$3CB1` selects direction from bit 5 alone.
            actor.state = if actor.sprite_flip.horizontal() { 2 } else { 1 };
            return Ok(Some(9));
        }
        1 | 2 => {
            if frame.every(2) {
                actor.screen.move_x(if actor.state == 1 { -3 } else { 3 });
                // Test the wrapped position after movement in either direction.
                if actor.screen.x >= 184 {
                    actor.sprite = 14;
                    actor.state = 0;
                }
            }
        }
        state => return Err(state),
    }
    Ok(None)
}

#[cfg(test)]
#[path = "tests/behavior.rs"]
mod tests;
