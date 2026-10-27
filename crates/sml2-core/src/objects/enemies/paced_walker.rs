//! Native paced walker enemy domain behavior, ROM `$59CE7` (anim 94 Mode A, anim 5 & 61 Mode B).
//!
//! Paces horizontally: walks for 48 pixels, pauses for 16 frames, then walks again.
//! If a wall is encountered, it immediately turns around.
//! Animates low sprite bit on even 4-frame cadence ($5B51A).
//!
//! Follows the idiomatic Rust porting guide: typed actor state, domain methods,
//! zero raw hex offsets or register plumbing.

use crate::actor::{
    animate_sprite_blink, ContactKind, FlashFlags, FrameTick, Position, SpriteFlipFlags,
};
use std::cmp::Ordering;

pub const WALK_BUDGET_PX: u8 = 48; // ROM $59D18 / $59D55: ld a, 48
pub const PAUSE_FRAMES: u8 = 16; // ROM $59D28: ld a, 16
pub const CONTACT_KIND: u8 = 76; // ROM $59CF3: ld a, 76 (0x4C)

/// Paced walker's native state; unknown imported values remain lossless until updated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PacedWalkerState {
    #[default]
    Init,
    WalkLeft,
    WalkRight,
    Unknown(u8),
}

/// Only the data this behavior owns; legacy slot bookkeeping stays in its adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PacedWalker {
    pub position: Position,
    pub sprite: u8,
    pub sprite_flip: SpriteFlipFlags,
    pub flash: FlashFlags,
    pub contact_kind: ContactKind,
    pub state: PacedWalkerState,
    pub remaining_pixels: u8,
    pub pause_ticks: u8,
}

pub struct PacedWalkerContext {
    pub frame: FrameTick,
    pub mario_ref_x: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacedWalkerEffect {
    None,
    Initialized { facing: Ordering },
    Turned { state: PacedWalkerState },
    Moved { state: PacedWalkerState },
    Paused,
}

pub trait PacedWalkerWorld {
    fn probe_left_wall(&mut self, position: Position) -> bool;
    fn probe_right_wall(&mut self, position: Position) -> bool;
}

/// Domain update for PacedWalker (ROM $59CE7).
pub fn update<W>(
    actor: &mut PacedWalker,
    context: PacedWalkerContext,
    world: &mut W,
) -> PacedWalkerEffect
where
    W: PacedWalkerWorld,
{
    // ROM $5B51A: toggle low bit of sprite every fourth frame
    animate_sprite_blink(&mut actor.sprite, context.frame, 4, 1);

    match actor.state {
        PacedWalkerState::Init | PacedWalkerState::Unknown(_) => {
            actor.flash = FlashFlags::from_bits(0x10);
            actor.contact_kind = ContactKind::from_raw(CONTACT_KIND);
            let facing = context.mario_ref_x.cmp(&actor.position.x);
            let (next_state, flip) = match facing {
                Ordering::Greater => (PacedWalkerState::WalkRight, true),
                Ordering::Less | Ordering::Equal => (PacedWalkerState::WalkLeft, false),
            };
            actor.state = next_state;
            actor.sprite_flip.set_horizontal(flip);
            actor.pause_ticks = 1;
            PacedWalkerEffect::Initialized { facing }
        }
        PacedWalkerState::WalkLeft => {
            if world.probe_left_wall(actor.position) {
                actor.sprite_flip.toggle_horizontal();
                actor.state = PacedWalkerState::WalkRight;
                PacedWalkerEffect::Turned {
                    state: PacedWalkerState::WalkRight,
                }
            } else {
                pause_step(actor, true)
            }
        }
        PacedWalkerState::WalkRight => {
            if world.probe_right_wall(actor.position) {
                actor.sprite_flip.toggle_horizontal();
                actor.state = PacedWalkerState::WalkLeft;
                PacedWalkerEffect::Turned {
                    state: PacedWalkerState::WalkLeft,
                }
            } else {
                pause_step(actor, false)
            }
        }
    }
}

fn pause_step(actor: &mut PacedWalker, left: bool) -> PacedWalkerEffect {
    if actor.pause_ticks != 0 {
        actor.pause_ticks = actor.pause_ticks.wrapping_sub(1);
        if actor.pause_ticks != 0 {
            return PacedWalkerEffect::None;
        }
        actor.remaining_pixels = WALK_BUDGET_PX;
        return PacedWalkerEffect::None;
    }

    if left {
        actor.position.move_x(-1);
    } else {
        actor.position.move_x(1);
    }

    let budget = actor.remaining_pixels.wrapping_sub(1);
    actor.remaining_pixels = budget;
    if budget != 0 {
        return PacedWalkerEffect::Moved {
            state: if left {
                PacedWalkerState::WalkLeft
            } else {
                PacedWalkerState::WalkRight
            },
        };
    }

    actor.pause_ticks = PAUSE_FRAMES;
    PacedWalkerEffect::Paused
}

#[cfg(test)]
#[path = "tests/paced_walker.rs"]
mod tests;
