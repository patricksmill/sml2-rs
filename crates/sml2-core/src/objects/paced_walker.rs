//! Native paced walker enemy domain behavior, ROM `$59CE7` (anim 94 Mode A, anim 5 & 61 Mode B).
//!
//! Paces horizontally: walks for 48 pixels, pauses for 16 frames, then walks again.
//! If a wall is encountered, it immediately turns around.
//! Animates low sprite bit on even 4-frame cadence ($5B51A).
//!
//! Follows the idiomatic Rust porting guide: typed actor state, domain methods,
//! zero raw hex offsets or register plumbing.

use crate::actor::{Actor, ContactKind, Facing, FlashFlags, FrameTick};

pub const WALK_BUDGET_PX: u8 = 48; // ROM $59D18 / $59D55: ld a, 48
pub const PAUSE_FRAMES: u8 = 16;   // ROM $59CEF / $59D28: ld a, 16
pub const CONTACT_KIND: u8 = 76;   // ROM $59CF3: ld a, 76 (0x4C)

/// Paced walker movement state ($59CD / $5D05 / $5D41: states 0..=2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacedWalkerState {
    Init = 0,
    WalkLeft = 1,
    WalkRight = 2,
}

impl PacedWalkerState {
    pub const fn from_raw(value: u8) -> Self {
        match value {
            1 => Self::WalkLeft,
            2 => Self::WalkRight,
            _ => Self::Init,
        }
    }

    pub const fn to_raw(self) -> u8 {
        self as u8
    }
}

pub struct PacedWalkerContext {
    pub frame: FrameTick,
    pub mario_ref_x: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacedWalkerEffect {
    None,
    Initialized { facing: u8 },
    Turned { state: PacedWalkerState },
    Moved { state: PacedWalkerState },
    Paused,
}

pub trait PacedWalkerWorld {
    fn probe_left_wall(&mut self, actor: &Actor) -> bool;
    fn probe_right_wall(&mut self, actor: &Actor) -> bool;
}

/// Domain update for PacedWalker (ROM $59CE7).
pub fn update<W>(
    actor: &mut Actor,
    context: PacedWalkerContext,
    world: &mut W,
) -> PacedWalkerEffect
where
    W: PacedWalkerWorld,
{
    // ROM $5B51A: toggle low bit of sprite every fourth frame
    actor.animate_two_frame_toggle(context.frame, 4);

    let state = PacedWalkerState::from_raw(actor.state);
    match state {
        PacedWalkerState::Init => {
            actor.flash = FlashFlags::from_bits(0x10);
            actor.contact_kind = ContactKind::from_raw(CONTACT_KIND);
            let (next_state, flip, facing) = match context.mario_ref_x.cmp(&actor.position.x) {
                std::cmp::Ordering::Greater => (PacedWalkerState::WalkRight.to_raw(), true, 1),
                std::cmp::Ordering::Less => (PacedWalkerState::WalkLeft.to_raw(), false, 0xFF),
                std::cmp::Ordering::Equal => (PacedWalkerState::WalkLeft.to_raw(), false, 0),
            };
            actor.state = next_state;
            actor.sprite_flip.set_horizontal(flip);
            actor.timer = 1;
            PacedWalkerEffect::Initialized { facing }
        }
        PacedWalkerState::WalkLeft => {
            if world.probe_left_wall(actor) {
                actor.sprite_flip.toggle_horizontal();
                actor.state = PacedWalkerState::WalkRight.to_raw();
                PacedWalkerEffect::Turned {
                    state: PacedWalkerState::WalkRight,
                }
            } else {
                pause_step(actor, true)
            }
        }
        PacedWalkerState::WalkRight => {
            if world.probe_right_wall(actor) {
                actor.sprite_flip.toggle_horizontal();
                actor.state = PacedWalkerState::WalkLeft.to_raw();
                PacedWalkerEffect::Turned {
                    state: PacedWalkerState::WalkLeft,
                }
            } else {
                pause_step(actor, false)
            }
        }
    }
}

fn pause_step(actor: &mut Actor, left: bool) -> PacedWalkerEffect {
    if actor.timer != 0 {
        actor.timer = actor.timer.wrapping_sub(1);
        if actor.timer != 0 {
            return PacedWalkerEffect::None;
        }
        actor.travel_direction = Facing::from_raw(WALK_BUDGET_PX);
        return PacedWalkerEffect::None;
    }

    if left {
        actor.position.move_x(-1);
    } else {
        actor.position.move_x(1);
    }

    let budget = actor.travel_direction.raw().wrapping_sub(1);
    actor.travel_direction = Facing::from_raw(budget);
    if budget != 0 {
        return PacedWalkerEffect::Moved {
            state: if left {
                PacedWalkerState::WalkLeft
            } else {
                PacedWalkerState::WalkRight
            },
        };
    }

    actor.timer = PAUSE_FRAMES;
    PacedWalkerEffect::Paused
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockWorld {
        left_wall: bool,
        right_wall: bool,
    }

    impl PacedWalkerWorld for MockWorld {
        fn probe_left_wall(&mut self, _actor: &Actor) -> bool {
            self.left_wall
        }
        fn probe_right_wall(&mut self, _actor: &Actor) -> bool {
            self.right_wall
        }
    }

    #[test]
    fn paced_walker_init_faces_mario_right_and_sets_flash_and_type() {
        let mut actor = Actor::default();
        actor.position.x = 100;
        let mut world = MockWorld {
            left_wall: false,
            right_wall: false,
        };
        let context = PacedWalkerContext {
            frame: FrameTick(1), // not multiple of 4, so sprite doesn't toggle
            mario_ref_x: 150,
        };

        let effect = update(&mut actor, context, &mut world);

        assert_eq!(actor.state, PacedWalkerState::WalkRight.to_raw());
        assert!(actor.sprite_flip.horizontal());
        assert_eq!(actor.flash.bits(), 0x10);
        assert_eq!(actor.contact_kind, ContactKind::from_raw(CONTACT_KIND));
        assert_eq!(actor.timer, 1);
        assert_eq!(effect, PacedWalkerEffect::Initialized { facing: 1 });
    }

    #[test]
    fn paced_walker_init_faces_mario_left_and_equal() {
        let mut actor = Actor::default();
        actor.position.x = 100;
        let mut world = MockWorld {
            left_wall: false,
            right_wall: false,
        };

        // Mario to the left
        let context_left = PacedWalkerContext {
            frame: FrameTick(1),
            mario_ref_x: 50,
        };
        let effect_left = update(&mut actor, context_left, &mut world);
        assert_eq!(actor.state, PacedWalkerState::WalkLeft.to_raw());
        assert!(!actor.sprite_flip.horizontal());
        assert_eq!(effect_left, PacedWalkerEffect::Initialized { facing: 0xFF });

        // Mario equal
        actor.state = 0;
        let context_eq = PacedWalkerContext {
            frame: FrameTick(1),
            mario_ref_x: 100,
        };
        let effect_eq = update(&mut actor, context_eq, &mut world);
        assert_eq!(actor.state, PacedWalkerState::WalkLeft.to_raw());
        assert!(!actor.sprite_flip.horizontal());
        assert_eq!(effect_eq, PacedWalkerEffect::Initialized { facing: 0 });
    }

    #[test]
    fn paced_walker_two_frame_toggle_cadence() {
        let mut actor = Actor::default();
        actor.sprite = 10;
        let mut world = MockWorld {
            left_wall: false,
            right_wall: false,
        };

        // Frame 0 (multiple of 4) -> toggles
        update(&mut actor, PacedWalkerContext { frame: FrameTick(0), mario_ref_x: 0 }, &mut world);
        assert_eq!(actor.sprite, 11);

        // Frame 1 -> no toggle
        update(&mut actor, PacedWalkerContext { frame: FrameTick(1), mario_ref_x: 0 }, &mut world);
        assert_eq!(actor.sprite, 11);

        // Frame 2 -> no toggle
        update(&mut actor, PacedWalkerContext { frame: FrameTick(2), mario_ref_x: 0 }, &mut world);
        assert_eq!(actor.sprite, 11);

        // Frame 3 -> no toggle
        update(&mut actor, PacedWalkerContext { frame: FrameTick(3), mario_ref_x: 0 }, &mut world);
        assert_eq!(actor.sprite, 11);

        // Frame 4 -> toggles back
        update(&mut actor, PacedWalkerContext { frame: FrameTick(4), mario_ref_x: 0 }, &mut world);
        assert_eq!(actor.sprite, 10);
    }

    #[test]
    fn paced_walker_cadence_counts_down_then_steps_48_times() {
        let mut actor = Actor::default();
        actor.position.x = 200;
        let mut world = MockWorld {
            left_wall: false,
            right_wall: false,
        };

        // Init: Mario to left -> WalkLeft, timer = 1
        update(
            &mut actor,
            PacedWalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 50,
            },
            &mut world,
        );
        assert_eq!(actor.timer, 1);
        assert_eq!(actor.position.x, 200);

        // Frame 1: timer 1 -> 0, arms budget to 48 (0x30), no movement
        let eff = update(
            &mut actor,
            PacedWalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 50,
            },
            &mut world,
        );
        assert_eq!(eff, PacedWalkerEffect::None);
        assert_eq!(actor.timer, 0);
        assert_eq!(actor.travel_direction.raw(), 48);
        assert_eq!(actor.position.x, 200);

        // Next 47 steps: each decrements budget and moves X by -1
        for i in 1..=47 {
            let eff = update(
                &mut actor,
                PacedWalkerContext {
                    frame: FrameTick(1),
                    mario_ref_x: 50,
                },
                &mut world,
            );
            assert_eq!(
                eff,
                PacedWalkerEffect::Moved {
                    state: PacedWalkerState::WalkLeft
                }
            );
            assert_eq!(actor.travel_direction.raw(), 48 - i);
            assert_eq!(actor.position.x, 200 - i as u16);
        }

        // 48th step: reaches budget 0, pauses with timer = 16 (0x10)
        let eff = update(
            &mut actor,
            PacedWalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 50,
            },
            &mut world,
        );
        assert_eq!(eff, PacedWalkerEffect::Paused);
        assert_eq!(actor.travel_direction.raw(), 0);
        assert_eq!(actor.position.x, 200 - 48);
        assert_eq!(actor.timer, 16);

        // Next 15 pause frames: timer decrements from 16 to 1, no movement
        for pause_step_idx in 1..=15 {
            let eff = update(
                &mut actor,
                PacedWalkerContext {
                    frame: FrameTick(1),
                    mario_ref_x: 50,
                },
                &mut world,
            );
            assert_eq!(eff, PacedWalkerEffect::None);
            assert_eq!(actor.timer, 16 - pause_step_idx);
            assert_eq!(actor.position.x, 200 - 48);
        }

        // 16th pause frame: timer 1 -> 0, reloads budget to 48, no movement
        let eff = update(
            &mut actor,
            PacedWalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 50,
            },
            &mut world,
        );
        assert_eq!(eff, PacedWalkerEffect::None);
        assert_eq!(actor.timer, 0);
        assert_eq!(actor.travel_direction.raw(), 48);
        assert_eq!(actor.position.x, 200 - 48);

        // Next frame: begins walking again
        let eff = update(
            &mut actor,
            PacedWalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 50,
            },
            &mut world,
        );
        assert_eq!(
            eff,
            PacedWalkerEffect::Moved {
                state: PacedWalkerState::WalkLeft
            }
        );
        assert_eq!(actor.position.x, 200 - 49);
    }

    #[test]
    fn paced_walker_wall_flips_direction() {
        let mut actor = Actor::default();
        actor.position.x = 100;
        actor.state = PacedWalkerState::WalkLeft.to_raw();
        actor.sprite_flip.set_horizontal(false);

        let mut world = MockWorld {
            left_wall: true,
            right_wall: false,
        };

        // Left wall hit -> turn right
        let eff = update(
            &mut actor,
            PacedWalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 0,
            },
            &mut world,
        );
        assert_eq!(
            eff,
            PacedWalkerEffect::Turned {
                state: PacedWalkerState::WalkRight
            }
        );
        assert_eq!(actor.state, PacedWalkerState::WalkRight.to_raw());
        assert!(actor.sprite_flip.horizontal());
        assert_eq!(actor.position.x, 100, "wall bounce does not step X");

        // Now facing right, right wall hit -> turn left
        world.left_wall = false;
        world.right_wall = true;
        let eff = update(
            &mut actor,
            PacedWalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 0,
            },
            &mut world,
        );
        assert_eq!(
            eff,
            PacedWalkerEffect::Turned {
                state: PacedWalkerState::WalkLeft
            }
        );
        assert_eq!(actor.state, PacedWalkerState::WalkLeft.to_raw());
        assert!(!actor.sprite_flip.horizontal());
        assert_eq!(actor.position.x, 100, "wall bounce does not step X");
    }
}
