//! Diagonal tracker: an easing horizontal leg, unit vertical steps, and a pause.
use crate::actor::{
    Actor, AnimationFrameError, ContactKind, FrameTick, MotionProfileError, Position,
};
use std::cmp::Ordering;

const SPRITE_DELTAS: [i8; 3] = [-2, 1, 1];

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
    Animation(AnimationFrameError),
    Motion(MotionProfileError),
}
impl From<AnimationFrameError> for Error {
    fn from(error: AnimationFrameError) -> Self {
        Self::Animation(error)
    }
}
impl From<MotionProfileError> for Error {
    fn from(error: MotionProfileError) -> Self {
        Self::Motion(error)
    }
}

pub(super) trait World {
    fn player_position(&self) -> Position;
    fn horizontal_profile(&self) -> &[u8];
    fn set_facing(&mut self, relation: Ordering);
}

pub(super) fn update(
    actor: &mut Actor,
    frame: FrameTick,
    world: &mut impl World,
) -> Result<(), Error> {
    let player = world.player_position();
    match actor.state {
        0 => {
            world.set_facing(actor.face_toward(player.x));
            actor.contact_kind = ContactKind::from_raw(77);
            return Ok(());
        }
        1 | 2 => {}
        state => return Err(Error::InvalidState(state)),
    }
    if actor.timer != 0 {
        actor.animate_sprite_delta_cycle(frame, 2, &SPRITE_DELTAS)?;
        actor.timer -= 1;
        if actor.timer != 0 {
            return Ok(());
        }
    }
    actor.animate_sprite_delta_cycle(frame, 1, &SPRITE_DELTAS)?;
    if !frame.every(2) {
        return Ok(());
    }

    let distance = i16::from(actor.motion_step(world.horizontal_profile(), 21)?);
    actor.position.move_x(if actor.state == 1 {
        -distance
    } else {
        distance
    });
    actor
        .position
        .move_y(if actor.motion_leg == 0 { 1 } else { -1 });
    if actor.motion_phase == 21 {
        actor.motion_phase = 0;
        world.set_facing(actor.face_toward(player.x));
        let vertical_relation = player.y.cmp(&actor.position.y);
        world.set_facing(vertical_relation);
        // $5B41A returns the target low byte on equality, followed by DEC A.
        let down = vertical_relation == Ordering::Greater
            || (vertical_relation == Ordering::Equal && player.y as u8 == 1);
        actor.motion_leg = u8::from(!down);
        actor.timer = 40;
    }
    Ok(())
}
