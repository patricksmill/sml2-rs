//! ROM `$59E5B` (mode B anim 11): a wall-turning walker that rises, then drops
//! back onto a new facing.
//!
//! State 0 arms flash 16 and contact 76, faces Mario, and loads a 48-frame
//! travel span. States 1/2 toggle the sprite every fourth frame, turn on the
//! side wall (`$84EA` / `$843C`, flipping with `state ^= 3`), and otherwise
//! walk one pixel until the span expires into state 3 with sprite 53. State 3
//! climbs the sprite to 55, then rises until the ceiling or 32 pixels. State 4
//! drops the sprite back to 53 and descends those pixels, then faces Mario.
use crate::actor::{Actor, ContactKind, FlashFlags, FrameTick};
use std::cmp::Ordering;

const SPAN: u8 = 48;
const RISE: u8 = 0x20;
const CLIMB_TOP: u8 = 0x37;
const DROP_SPRITE: u8 = 0x35;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Facing(Ordering),
    Continue,
}

pub(super) trait World {
    fn left_wall(&mut self, actor: &Actor) -> bool;
    fn right_wall(&mut self, actor: &Actor) -> bool;
    fn ceiling(&mut self, actor: &Actor) -> bool;
}

pub(super) fn update(
    actor: &mut Actor,
    frame: FrameTick,
    player_x: u16,
    world: &mut impl World,
) -> Result<Effect, Error> {
    if actor.state == 0 {
        actor.flash = FlashFlags::from_bits(16);
        actor.contact_kind = ContactKind::from_raw(76);
        let relation = actor.face_toward(player_x);
        actor.set_travel_span(SPAN);
        return Ok(Effect::Facing(relation));
    }
    match actor.state {
        1 => walk(actor, frame, world, true),
        2 => walk(actor, frame, world, false),
        3 => rise(actor, frame, world),
        4 => descend(actor, frame, player_x),
        state => Err(Error::InvalidState(state)),
    }
}

fn walk(
    actor: &mut Actor,
    frame: FrameTick,
    world: &mut impl World,
    left: bool,
) -> Result<Effect, Error> {
    actor.animate_two_frame_toggle(frame, 4);
    let blocked = if left {
        world.left_wall(actor)
    } else {
        world.right_wall(actor)
    };
    if blocked {
        actor.sprite_flip.toggle_horizontal();
        actor.state ^= 3;
        return Ok(Effect::Continue);
    }
    actor.position.move_x(if left { -1 } else { 1 });
    let remaining = actor.travel_span().wrapping_sub(1);
    actor.set_travel_span(remaining);
    if remaining != 0 {
        return Ok(Effect::Continue);
    }
    actor.set_travel_span(SPAN);
    actor.state = 3;
    actor.sprite = 0x35;
    Ok(Effect::Continue)
}

fn rise(actor: &mut Actor, frame: FrameTick, world: &mut impl World) -> Result<Effect, Error> {
    if !frame.every(2) {
        return Ok(Effect::Continue);
    }
    if actor.sprite != CLIMB_TOP {
        actor.sprite = actor.sprite.wrapping_add(1);
        return Ok(Effect::Continue);
    }
    if !world.ceiling(actor) {
        actor.position.move_y(-1);
        actor.hit_reaction = actor.hit_reaction.wrapping_add(1);
        if actor.hit_reaction != RISE {
            return Ok(Effect::Continue);
        }
    }
    actor.state = actor.state.wrapping_add(1);
    Ok(Effect::Continue)
}

fn descend(actor: &mut Actor, frame: FrameTick, player_x: u16) -> Result<Effect, Error> {
    if !frame.every(2) {
        return Ok(Effect::Continue);
    }
    if actor.sprite != DROP_SPRITE {
        actor.sprite = actor.sprite.wrapping_sub(1);
        return Ok(Effect::Continue);
    }
    actor.position.move_y(1);
    actor.hit_reaction = actor.hit_reaction.wrapping_sub(1);
    if actor.hit_reaction != 0 {
        return Ok(Effect::Continue);
    }
    Ok(Effect::Facing(actor.face_toward(player_x)))
}
