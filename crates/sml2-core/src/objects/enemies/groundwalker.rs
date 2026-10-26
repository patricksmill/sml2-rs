//! Native groundwalker enemy domain behavior, ROM `$59BC5` (anim 88).
//!
//! Follows the idiomatic Rust porting guide: typed actor state, domain methods,
//! zero raw hex offsets or register plumbing.

use crate::actor::{
    animate_four_step_bob, ContactKind, FlashFlags, FrameTick, Position, SpriteFlipFlags,
};
use std::cmp::Ordering;

/// Groundwalker's native state; unknown imported values remain lossless until updated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GroundWalkerState {
    #[default]
    Init,
    WalkLeft,
    WalkRight,
    Unknown(u8),
}

/// Only the data this behavior owns; legacy slot bookkeeping stays in its adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GroundWalker {
    pub position: Position,
    pub sprite: u8,
    pub sprite_flip: SpriteFlipFlags,
    pub flash: FlashFlags,
    pub contact_kind: ContactKind,
    pub animation_frame: u8,
    pub state: GroundWalkerState,
    pub remaining_pixels: u8,
    pub pause_ticks: u8,
}

pub struct GroundWalkerContext {
    pub frame: FrameTick,
    pub mario_ref_x: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroundWalkerEffect {
    None,
    Initialized { facing: Ordering },
    Turned { state: GroundWalkerState },
    Moved { state: GroundWalkerState },
    Paused,
}

pub trait GroundWalkerWorld {
    fn probe_left_wall(&mut self, position: Position) -> bool;
    fn probe_right_wall(&mut self, position: Position) -> bool;
}

/// Domain update for GroundWalker (ROM $59BC5, anim 88).
pub fn update<W>(
    actor: &mut GroundWalker,
    context: GroundWalkerContext,
    world: &mut W,
) -> GroundWalkerEffect
where
    W: GroundWalkerWorld,
{
    // $5B585: four-step bob at period 2
    let _ = animate_four_step_bob(
        &mut actor.sprite,
        &mut actor.animation_frame,
        context.frame,
        2,
    );

    match actor.state {
        GroundWalkerState::Init | GroundWalkerState::Unknown(_) => {
            actor.flash = FlashFlags::from_bits(0x10);
            actor.contact_kind = ContactKind::from_raw(0x4E);
            let facing = context.mario_ref_x.cmp(&actor.position.x);
            let (next_state, flip) = match facing {
                Ordering::Greater => (GroundWalkerState::WalkRight, true),
                Ordering::Less | Ordering::Equal => (GroundWalkerState::WalkLeft, false),
            };
            actor.state = next_state;
            actor.sprite_flip.set_horizontal(flip);
            actor.pause_ticks = 1;
            GroundWalkerEffect::Initialized { facing }
        }
        GroundWalkerState::WalkLeft => {
            if world.probe_left_wall(actor.position) {
                actor.sprite_flip.toggle_horizontal();
                actor.state = GroundWalkerState::WalkRight;
                GroundWalkerEffect::Turned {
                    state: GroundWalkerState::WalkRight,
                }
            } else {
                pause_step(actor, true)
            }
        }
        GroundWalkerState::WalkRight => {
            if world.probe_right_wall(actor.position) {
                actor.sprite_flip.toggle_horizontal();
                actor.state = GroundWalkerState::WalkLeft;
                GroundWalkerEffect::Turned {
                    state: GroundWalkerState::WalkLeft,
                }
            } else {
                pause_step(actor, false)
            }
        }
    }
}

fn pause_step(actor: &mut GroundWalker, left: bool) -> GroundWalkerEffect {
    if actor.pause_ticks != 0 {
        actor.pause_ticks = actor.pause_ticks.wrapping_sub(1);
        if actor.pause_ticks != 0 {
            return GroundWalkerEffect::None;
        }
        actor.remaining_pixels = 0x30;
        return GroundWalkerEffect::None;
    }

    if left {
        actor.position.move_x(-1);
    } else {
        actor.position.move_x(1);
    }

    let budget = actor.remaining_pixels.wrapping_sub(1);
    actor.remaining_pixels = budget;
    if budget != 0 {
        return GroundWalkerEffect::Moved {
            state: if left {
                GroundWalkerState::WalkLeft
            } else {
                GroundWalkerState::WalkRight
            },
        };
    }

    actor.pause_ticks = 0x10;
    GroundWalkerEffect::Paused
}

#[cfg(test)]
#[path = "tests/groundwalker.rs"]
mod tests;
