//! Native Left Marcher domain behavior, ROM `$591DE` (anim 57 Mode A & B).
//!
//! Follows the idiomatic Rust porting guide: typed actor state, domain methods,
//! zero raw hex offsets or register plumbing.

use crate::actor::{Actor, ContactKind, FrameTick};

/// Left Marcher movement state machine (ROM $591DE).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeftMarcherState {
    Init = 0,
    Marching = 1,
}

impl LeftMarcherState {
    pub const fn from_raw(value: u8) -> Self {
        match value {
            1 => Self::Marching,
            _ => Self::Init,
        }
    }

    pub const fn to_raw(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeftMarcherEffect {
    Initialized,
    Marching,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeftMarcherContext {
    pub frame: FrameTick,
}

/// Helper for ROM `$5B5B6`: 3-step sprite crawl/bob cycle (+1, +1, -2) on even frames.
///
/// In ROM at `$5B5B6`:
/// - Gate: `ldh a, [$FFF0]; and $01; ret nz` (even frames only: period 2)
/// - `animation_frame = (animation_frame + 1) % 3`
/// - Delta table at `$75D3`: `[-2, +1, +1]` (`[0xFE, 0x01, 0x01]`)
/// - `sprite += table[animation_frame]`
#[inline]
pub fn advance_three_step_crawl(actor: &mut Actor, frame: FrameTick) {
    if !frame.every(2) {
        return;
    }
    let next_phase = actor.animation_frame.wrapping_add(1) % 3;
    actor.animation_frame = next_phase;
    let delta: i8 = match next_phase {
        1 => 1,
        2 => 1,
        _ => -2,
    };
    actor.sprite = (actor.sprite as i16 + delta as i16) as u8;
}

/// Domain update for Left Marcher (ROM $591DE, anim 57).
pub fn update(actor: &mut Actor, context: LeftMarcherContext) -> LeftMarcherEffect {
    // $5B5B6: 3-step crawl bob on even frames
    advance_three_step_crawl(actor, context.frame);

    let state = LeftMarcherState::from_raw(actor.state);
    match state {
        LeftMarcherState::Init => {
            // $591E6..$591EC: one-shot initialization
            actor.contact_kind = ContactKind::from_raw(77); // 0x4D standard stompable obstacle
            actor.state = LeftMarcherState::Marching.to_raw();
            LeftMarcherEffect::Initialized
        }
        LeftMarcherState::Marching => {
            // $591F3..$591F8: advance 1 px left (X -= 1)
            actor.position.move_x(-1);
            LeftMarcherEffect::Marching
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_sets_contact_and_state() {
        let mut actor = Actor::default();
        actor.position.x = 200;
        actor.position.y = 100;
        actor.sprite = 104;
        actor.state = LeftMarcherState::Init.to_raw();

        // Odd frame: no sprite bob advance
        let effect = update(
            &mut actor,
            LeftMarcherContext {
                frame: FrameTick::new(1),
            },
        );
        assert_eq!(effect, LeftMarcherEffect::Initialized);
        assert_eq!(actor.state, LeftMarcherState::Marching.to_raw());
        assert_eq!(actor.contact_kind.raw(), 77); // 0x4D
        assert_eq!(actor.position.x, 200, "init frame does not move");
        assert_eq!(actor.sprite, 104);
        assert_eq!(actor.animation_frame, 0);
    }

    #[test]
    fn test_marching_movement() {
        let mut actor = Actor::default();
        actor.position.x = 200;
        actor.position.y = 100;
        actor.sprite = 104;
        actor.state = LeftMarcherState::Marching.to_raw();

        // Frame 1 (odd): moves 1 px left
        let effect = update(
            &mut actor,
            LeftMarcherContext {
                frame: FrameTick::new(1),
            },
        );
        assert_eq!(effect, LeftMarcherEffect::Marching);
        assert_eq!(actor.position.x, 199);
        assert_eq!(actor.sprite, 104);

        // Frame 3 (odd): moves 1 px left
        let effect2 = update(
            &mut actor,
            LeftMarcherContext {
                frame: FrameTick::new(3),
            },
        );
        assert_eq!(effect2, LeftMarcherEffect::Marching);
        assert_eq!(actor.position.x, 198);
        assert_eq!(actor.sprite, 104);
    }

    #[test]
    fn test_three_step_sprite_cycle() {
        let mut actor = Actor::default();
        actor.sprite = 104;
        actor.animation_frame = 0;

        // Step 1: even frame -> animation_frame 0 -> 1, delta = +1
        advance_three_step_crawl(&mut actor, FrameTick::new(2));
        assert_eq!(actor.animation_frame, 1);
        assert_eq!(actor.sprite, 105);

        // Odd frame: no change
        advance_three_step_crawl(&mut actor, FrameTick::new(3));
        assert_eq!(actor.animation_frame, 1);
        assert_eq!(actor.sprite, 105);

        // Step 2: even frame -> animation_frame 1 -> 2, delta = +1
        advance_three_step_crawl(&mut actor, FrameTick::new(4));
        assert_eq!(actor.animation_frame, 2);
        assert_eq!(actor.sprite, 106);

        // Step 3: even frame -> animation_frame 2 -> 0 (mod 3), delta = -2
        advance_three_step_crawl(&mut actor, FrameTick::new(6));
        assert_eq!(actor.animation_frame, 0);
        assert_eq!(actor.sprite, 104);

        // Step 4: even frame -> cycles back: animation_frame 0 -> 1, delta = +1
        advance_three_step_crawl(&mut actor, FrameTick::new(8));
        assert_eq!(actor.animation_frame, 1);
        assert_eq!(actor.sprite, 105);
    }
}
