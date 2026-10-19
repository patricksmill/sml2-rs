//! ROM `$5966C` (mode A anim 80): a two-way patroller with a flashing pause.
//!
//! State 0 faces Mario, arms contact 72, flash 16, a 48-frame leg timer (`+25`)
//! and lifts 4 px. States 1/2 flash the palette, step one pixel in their
//! direction and count the leg timer down; reaching zero reloads 96 and flips
//! to the opposite state (`state ^= 3`), after a 16-frame palette flash.
use crate::actor::{Actor, ContactKind, FlashFlags, FrameTick};

const CONTACT: u8 = 72;
const LEG: u8 = 48;
const RELOAD: u8 = 96;
const FLASH_TIME: u8 = 16;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick, player_x: u16) -> Result<(), Error> {
    match actor.state {
        // $59670
        0 => {
            actor.contact_kind = ContactKind::from_raw(CONTACT);
            actor.travel_direction = crate::actor::Facing::from_raw(LEG);
            actor.face_toward(player_x);
            actor.flash = FlashFlags::from_bits(16);
            actor.position.move_y(-4);
            Ok(())
        }
        // $5968E: left leg.
        1 => {
            pause_flash(actor, frame);
            actor.position.move_x(-1);
            leg_tick(actor);
            Ok(())
        }
        // $596BA: right leg.
        2 => {
            pause_flash(actor, frame);
            actor.position.move_x(1);
            leg_tick(actor);
            Ok(())
        }
        state => Err(Error::InvalidState(state)),
    }
}

/// `$5968E`/`$596BA` head: while the 16-frame pause runs, blink the palette.
fn pause_flash(actor: &mut Actor, frame: FrameTick) {
    if actor.timer == 0 {
        return;
    }
    actor.flash.flash_palette_on_tick(frame, 2);
    actor.timer = actor.timer.wrapping_sub(1);
    if actor.timer == 0 {
        actor.flash = FlashFlags::from_bits(16);
    }
}

/// `$596A0`: count the 48-frame leg; at zero reload and swap direction.
fn leg_tick(actor: &mut Actor) {
    actor.travel_direction =
        crate::actor::Facing::from_raw(actor.travel_direction.raw().wrapping_sub(1));
    if actor.travel_direction.raw() != 0 {
        return;
    }
    actor.travel_direction = crate::actor::Facing::from_raw(RELOAD);
    actor.timer = FLASH_TIME;
    actor.state ^= 3;
}
