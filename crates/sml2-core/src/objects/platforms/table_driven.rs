//! ROM `$597BD` (mode A anim 85): an object that loops along a table-driven
//! closed curve.
//!
//! State 0 arms contact 72, faces Mario (`$5B3C6`) and starts both curve
//! cursors at 1. Every call runs the four-step sprite cycle on every fourth
//! frame (`$5B57E`); movement then runs on even frames only. Each of the
//! eight states moves Y with the `$FFE8` cursor and X with the `$FFE9` cursor,
//! one table sample per call, and hands off when the X cursor completes:
//!
//! | State | Y              | X         | Next |
//! |-------|----------------|-----------|------|
//! | 1/2   | up `$5938`     | -/+ `$594E` | 3/4 |
//! | 3/4   | down `$5938`   | -/+ `$5964` | 5/6, flip |
//! | 5/6   | down `$597A`   | +/- `$5990` | 7/8 |
//! | 7/8   | up `$597A`     | +/- `$5990` (X cursor counts down) | 2/1 |
//!
//! The source quirks are kept: state 3 does not reset the Y cursor while
//! state 4 does, and the cursor helpers clamp or wrap differently
//! (`$58CE` clamps at 20, `$5919` wraps to 1, the down-counters stop at 1).
use crate::actor::{Actor, ContactKind, FrameTick};

pub(super) const CONTACT: u8 = 72;

/// `$5B5A2`: sprite deltas for the `$5B58A` cycle.
const SPRITE_DELTAS: [i8; 4] = [-2, 1, -1, 2];

/// ROM `$59938-$599A5`: five 22-byte curves, kept contiguous because the
/// source indexes them with unchecked byte cursors.
const CURVES: [u8; 110] = [
    // $5938: Y, states 1-4
    128, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 0, 1, 1, 0, 1, 0, 0, 0, 1, 128,
    // $594E: X, states 1/2
    128, 0, 0, 0, 1, 0, 1, 1, 1, 1, 2, 1, 1, 2, 2, 1, 2, 2, 2, 2, 2, 128,
    // $5964: X, states 3/4
    128, 1, 2, 2, 2, 2, 1, 2, 2, 1, 1, 2, 1, 1, 1, 1, 0, 1, 0, 0, 1, 128,
    // $597A: Y, states 5-8
    128, 0, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 3, 2, 3, 2, 2, 2, 0, 128,
    // $5990: X, states 5-8
    128, 0, 0, 0, 0, 0, 1, 0, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 128,
];
const Y_OUTER: usize = 0x00; // $5938
const X_FIRST: usize = 0x16; // $594E
const X_SECOND: usize = 0x2C; // $5964
const Y_INNER: usize = 0x42; // $597A
const X_INNER: usize = 0x58; // $5990

/// Both cursors complete a leg on reaching this value.
const LEG_END: u8 = 21;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
    CursorOutOfRange { table: usize, index: u8 },
    Animation,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized,
    Moved,
    Waiting,
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick, player_x: u16) -> Result<Effect, Error> {
    if actor.state == 0 {
        actor.contact_kind = ContactKind::from_raw(CONTACT);
        actor.face_toward(player_x);
        actor.motion_leg = 1;
        actor.set_travel_span(1);
        return Ok(Effect::Initialized);
    }
    actor
        .animate_sprite_delta_cycle(frame, 4, &SPRITE_DELTAS)
        .map_err(|_| Error::Animation)?;
    if !frame.every(2) {
        return Ok(Effect::Waiting);
    }
    match actor.state {
        // $597EE / $5985C
        1 | 2 => {
            rise_outer(actor)?;
            move_x(actor, X_FIRST, actor.state == 2)?;
            if advance_x(actor) {
                actor.set_travel_span(1);
                actor.state += 2;
            }
        }
        // $59806 / $59874
        3 | 4 => {
            fall_outer(actor)?;
            move_x(actor, X_SECOND, actor.state == 4)?;
            if advance_x(actor) {
                actor.set_travel_span(1);
                if actor.state == 4 {
                    actor.motion_leg = 1;
                }
                actor.state += 2;
                actor.animation_frame = 0;
                actor.sprite_flip.toggle_horizontal();
            }
        }
        // $59827 / $59899
        5 | 6 => {
            fall_inner(actor)?;
            move_x(actor, X_INNER, actor.state == 5)?;
            if advance_x(actor) {
                actor.set_travel_span(20);
                actor.motion_leg = 20;
                actor.state += 2;
            }
        }
        // $59843 / $598B5
        7 | 8 => {
            rise_inner(actor)?;
            move_x(actor, X_INNER, actor.state == 7)?;
            let count = actor.travel_span().wrapping_sub(1);
            actor.set_travel_span(count);
            if count == 0 {
                actor.set_travel_span(1);
                actor.motion_leg = 1;
                actor.state = if actor.state == 7 { 2 } else { 1 };
            }
        }
        state => return Err(Error::InvalidState(state)),
    }
    Ok(Effect::Moved)
}

fn sample(table: usize, index: u8) -> Result<u8, Error> {
    CURVES
        .get(table + usize::from(index))
        .copied()
        .ok_or(Error::CursorOutOfRange { table, index })
}

/// `$58EA` (`$3D32`) / `$590E` (`$3D22`): X by the `$FFE9` sample.
fn move_x(actor: &mut Actor, table: usize, right: bool) -> Result<(), Error> {
    let step = sample(table, actor.travel_span())?;
    actor.position.x = if right {
        actor.position.x.wrapping_add(u16::from(step))
    } else {
        actor.position.x.wrapping_sub(u16::from(step))
    };
    Ok(())
}

/// `inc [$FFE9]; cp 21`: true when the X leg completes.
fn advance_x(actor: &mut Actor) -> bool {
    let count = actor.travel_span().wrapping_add(1);
    actor.set_travel_span(count);
    count == LEG_END
}

fn rise(actor: &mut Actor, table: usize) -> Result<(), Error> {
    let step = sample(table, actor.motion_leg)?;
    actor.position.y = actor.position.y.wrapping_sub(u16::from(step));
    Ok(())
}

fn fall(actor: &mut Actor, table: usize) -> Result<(), Error> {
    let step = sample(table, actor.motion_leg)?;
    actor.position.y = actor.position.y.wrapping_add(u16::from(step));
    Ok(())
}

/// `$58CE`: rise on `$5938`; the cursor counts up and holds at 20.
fn rise_outer(actor: &mut Actor) -> Result<(), Error> {
    rise(actor, Y_OUTER)?;
    actor.motion_leg = actor.motion_leg.wrapping_add(1);
    if actor.motion_leg == LEG_END {
        actor.motion_leg = 20;
    }
    Ok(())
}

/// `$58F5`: fall on `$5938`; the cursor counts down and holds at 1.
fn fall_outer(actor: &mut Actor) -> Result<(), Error> {
    fall(actor, Y_OUTER)?;
    count_down_to_one(actor);
    Ok(())
}

/// `$5919`: fall on `$597A`; the cursor counts up and wraps to 1.
fn fall_inner(actor: &mut Actor) -> Result<(), Error> {
    fall(actor, Y_INNER)?;
    actor.motion_leg = actor.motion_leg.wrapping_add(1);
    if actor.motion_leg == LEG_END {
        actor.motion_leg = 1;
    }
    Ok(())
}

/// `$592A`: rise on `$597A`; the cursor counts down and holds at 1.
fn rise_inner(actor: &mut Actor) -> Result<(), Error> {
    rise(actor, Y_INNER)?;
    count_down_to_one(actor);
    Ok(())
}

fn count_down_to_one(actor: &mut Actor) {
    actor.motion_leg = actor.motion_leg.wrapping_sub(1);
    if actor.motion_leg == 0 {
        actor.motion_leg = 1;
    }
}
