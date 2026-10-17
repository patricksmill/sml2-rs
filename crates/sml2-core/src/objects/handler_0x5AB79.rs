//! ROM `$5AB79` (mode B anim 100): a flying ant.
//!
//! Every call bobs on even frames (`$5B585`). State 0 arms contact 74, faces
//! Mario, and remembers that state. States 1/2 fall two pixels until the
//! ground, then climb into states 3/4. States 3/4 rise two pixels until the
//! ceiling. Each state also walks two pixels and turns on the leading wall.
//! State changes are copied into the visibility mask.
use crate::actor::{Actor, ContactKind, FrameTick};
use std::cmp::Ordering;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized { facing: Ordering },
    Continue,
}

pub(super) trait World {
    fn grounded(&mut self, actor: &Actor) -> bool;
    fn ceiling(&mut self, actor: &Actor) -> bool;
    fn left_wall(&mut self, actor: &Actor) -> bool;
    fn right_wall(&mut self, actor: &Actor) -> bool;
}

pub(super) fn update(
    actor: &mut Actor,
    frame: FrameTick,
    player_x: u16,
    world: &mut impl World,
) -> Result<Effect, Error> {
    let _ = actor.animate_four_step_bob_cadence(frame, 2);
    match actor.state {
        0 => {
            actor.contact_kind = ContactKind::from_raw(74);
            let facing = actor.face_toward(player_x);
            actor.visibility_mask = actor.state;
            Ok(Effect::Initialized { facing })
        }
        1 => {
            fall_or_land(actor, world);
            walk(actor, world, true);
            Ok(Effect::Continue)
        }
        2 => {
            fall_or_land(actor, world);
            walk(actor, world, false);
            Ok(Effect::Continue)
        }
        3 => {
            rise_or_cling(actor, world);
            walk(actor, world, true);
            Ok(Effect::Continue)
        }
        4 => {
            rise_or_cling(actor, world);
            walk(actor, world, false);
            Ok(Effect::Continue)
        }
        state => Err(Error::InvalidState(state)),
    }
}

fn remember(actor: &mut Actor) {
    actor.visibility_mask = actor.state;
}

fn fall_or_land(actor: &mut Actor, world: &mut impl World) {
    if world.grounded(actor) {
        actor.state = actor.state.wrapping_add(2);
        remember(actor);
    } else {
        actor.position.move_y(2);
    }
}

fn rise_or_cling(actor: &mut Actor, world: &mut impl World) {
    if world.ceiling(actor) {
        actor.state = actor.state.wrapping_sub(2);
        remember(actor);
    } else {
        actor.position.move_y(-2);
    }
}

fn walk(actor: &mut Actor, world: &mut impl World, left: bool) {
    let blocked = if left {
        world.left_wall(actor)
    } else {
        world.right_wall(actor)
    };
    if blocked {
        actor.state = if left {
            actor.state.wrapping_add(1)
        } else {
            actor.state.wrapping_sub(1)
        };
        remember(actor);
    } else {
        actor.position.move_x(if left { -2 } else { 2 });
    }
}
