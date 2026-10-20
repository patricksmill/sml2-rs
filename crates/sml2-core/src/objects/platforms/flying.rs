//! ROM `$5A545` (mode B anim 68): a flying platform.
//!
//! State 0 arms contact 32 and flash 16, faces Mario, and spawns anim 69.
//! Later calls bob the sprite every eighth frame (`$7603`). On even frames,
//! states 1/2 walk until a side wall, carrying a rider, and bob vertically on
//! the `$74AB` / `$7488` curves. A wall flips direction with `state ^= 3`.
use crate::actor::{Actor, ContactKind, FlashFlags, FrameTick, SpriteFlipFlags};
use std::cmp::Ordering;

const RISE: [u8; 9] = [2, 1, 2, 1, 1, 1, 0, 0, 0];
const FALL: [u8; 9] = [0, 0, 1, 1, 1, 2, 1, 2, 2];
const BOB: [i8; 4] = [-3, 1, 1, 1];

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized {
        facing: Ordering,
        sprite: u8,
        anim: u8,
        flip: SpriteFlipFlags,
    },
    Moved {
        carry_dx: Option<i16>,
        carry_dy: Option<i16>,
    },
    Idle,
}

pub(super) trait World {
    fn left_blocked(&mut self, actor: &Actor) -> bool;
    fn right_blocked(&mut self, actor: &Actor) -> bool;
}

pub(super) fn update(
    actor: &mut Actor,
    frame: FrameTick,
    player_x: u16,
    world: &mut impl World,
) -> Result<Effect, Error> {
    if actor.state == 0 {
        actor.contact_kind = ContactKind::from_raw(32);
        actor.flash = FlashFlags::from_bits(16);
        let facing = actor.face_toward(player_x);
        return Ok(Effect::Initialized {
            facing,
            sprite: 168,
            anim: 69,
            flip: actor.sprite_flip,
        });
    }
    bob(actor, frame);
    if !frame.every(2) {
        return Ok(Effect::Idle);
    }
    let left = match actor.state {
        1 => true,
        2 => false,
        state => return Err(Error::InvalidState(state)),
    };
    let blocked = if left {
        world.left_blocked(actor)
    } else {
        world.right_blocked(actor)
    };
    if blocked {
        actor.sprite_flip.toggle_horizontal();
        actor.state ^= 3;
        return Ok(Effect::Idle);
    }
    let dx = if left { -1 } else { 1 };
    actor.position.move_x(dx);
    let dy = vertical(actor);
    let riding = actor.interactions.stood_on();
    Ok(Effect::Moved {
        carry_dx: riding.then_some(dx),
        carry_dy: riding.then_some(dy),
    })
}

fn bob(actor: &mut Actor, frame: FrameTick) {
    if !frame.every(8) {
        return;
    }
    actor.animation_frame = actor.animation_frame.wrapping_add(1);
    if actor.animation_frame == 4 {
        actor.animation_frame = 0;
    }
    let delta = BOB[usize::from(actor.animation_frame)];
    actor.sprite = actor.sprite.wrapping_add_signed(delta);
}

fn vertical(actor: &mut Actor) -> i16 {
    let rising = actor.motion_leg == 0;
    let table = if rising { &RISE } else { &FALL };
    let phase = usize::from(actor.motion_phase);
    let step = i16::from(*table.get(phase).unwrap_or(&0));
    if actor.motion_phase != 8 {
        actor.motion_phase = actor.motion_phase.wrapping_add(1);
    }
    let dy = if rising { -step } else { step };
    actor.position.move_y(dy);
    if actor.motion_phase == 8 {
        actor.motion_phase = 0;
        actor.motion_leg ^= 1;
    }
    dy
}
