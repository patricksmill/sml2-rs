//! Native generic enemy reactions from bank $16: the stun and revive (`$5834F`),
//! the defeat hop and fall (`$581B1`) and the knockback after a hit
//! (`$411A..$4185`, landing at `$5819B`). Slot/world mapping lives in the
//! `walkers` adapter; sprite re-derivation and slot removal stay engine work.
use std::cmp::Ordering;
use std::ops::Range;

use super::falling_dropper::step_fall_speed;
use super::walker::WalkerWorld;
use super::BOUNCE_SPEED;
use crate::actor::{Actor, ContactKind, FrameTick, MotionProfileError};

/// Frames a stunned enemy lies still before it spins back up.
const STUN_FRAMES: u8 = 192;
/// Sprite flips in the revive spin.
const REVIVE_SPIN_STEPS: u8 = 8;
/// Contact kind while stunned.
const STUNNED_CONTACT: u8 = 0x06;
/// Screen Y below which a defeated enemy has fallen out of view.
pub const DEFEAT_OFFSCREEN_Y: u8 = 0xA0;
/// The knockback bounce ends at this phase.
const BOUNCE_CAP: u8 = 15;

#[derive(Debug, PartialEq, Eq)]
pub enum StunEffect {
    None,
    /// The spin finished as definition 1; the host re-derives its sprite.
    Revive,
}

/// `$5834F`: lie stunned (falling while airborne), spin, then revive.
pub fn update_stun<W: WalkerWorld>(
    actor: &mut Actor,
    frame: FrameTick,
    world: &mut W,
) -> StunEffect {
    match actor.state {
        // `$58364`: count down; fall when airborne.
        1 => {
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer == 0 {
                actor.state = 2;
                return StunEffect::None;
            }
            if world.probe_ground(actor) {
                return StunEffect::None;
            }
            let step = step_fall_speed(actor);
            actor.position.move_y(i16::from(step));
        }
        // `$58380`: spin every fourth frame.
        2 => {
            if !frame.every(4) {
                return StunEffect::None;
            }
            actor.sprite ^= 0x0B;
            actor.timer = actor.timer.wrapping_add(1);
            if actor.timer == REVIVE_SPIN_STEPS {
                actor.definition = 1;
                return StunEffect::Revive;
            }
        }
        _ => {
            actor.contact_kind = ContactKind::from_raw(STUNNED_CONTACT);
            actor.state = 1;
            actor.timer = STUN_FRAMES;
        }
    }
    StunEffect::None
}

#[derive(Debug, PartialEq, Eq)]
pub enum DefeatEffect {
    None,
    /// Fell off the bottom of the screen: remove the slot in remove mode 2.
    Remove,
}

/// `$581B1`: hop, rise, then fall at double speed until off screen.
/// `view_bounds` is an enlarged view's world-Y band (see
/// `ObjectSim::vertical_reset_bounds`); `None` keeps the native byte test.
pub fn update_defeat(
    actor: &mut Actor,
    view_bounds: Option<Range<i32>>,
) -> Result<DefeatEffect, MotionProfileError> {
    match actor.state {
        1 => actor.advance_defeat_rise()?,
        // `$581E2`: the screen Y tested is the previous frame's projection.
        2 => {
            let offscreen = view_bounds.map_or(actor.screen.y >= DEFEAT_OFFSCREEN_Y, |b| {
                !b.contains(&i32::from(actor.position.y))
            });
            let step = step_fall_speed(actor).wrapping_mul(2);
            actor.position.move_y(i16::from(step));
            if offscreen {
                return Ok(DefeatEffect::Remove);
            }
        }
        _ => actor.begin_defeat_hop(),
    }
    Ok(DefeatEffect::None)
}

#[derive(Debug, PartialEq, Eq)]
pub enum KnockbackEffect {
    None,
    /// The knockback began facing away from Mario; the ordering is his side.
    Faced(Ordering),
    /// Landed: the host re-derives the sprite, then applies [`resume_walk`]
    /// when `resume` holds the contact kind to keep.
    Landed {
        resume: Option<ContactKind>,
    },
}

/// Bounce-curve step clamped at its cap, as `$5B451`'s sibling picker reads it.
fn step_bounce_speed(actor: &mut Actor) -> u8 {
    let speed = BOUNCE_SPEED[usize::from(actor.motion_phase.min(BOUNCE_CAP))];
    if actor.motion_phase != BOUNCE_CAP {
        actor.motion_phase = actor.motion_phase.wrapping_add(1);
    }
    speed
}

/// The knockback after a hit: bounce away from Mario, then fall until landing.
/// States 1/2 bounce right/left, 3/4 fall right/left; a wall flips the side.
pub fn update_knockback<W: WalkerWorld>(
    actor: &mut Actor,
    mario_ref_x: u16,
    world: &mut W,
) -> KnockbackEffect {
    if actor.state == 0 {
        let side = actor.face_toward(mario_ref_x);
        actor.sprite_flip.toggle_horizontal();
        return KnockbackEffect::Faced(side);
    }
    let leg = actor.state.wrapping_sub(1);
    if leg == 0 || leg == 2 {
        actor.position.move_x(1);
        if world.probe_right_wall(actor) {
            actor.state = actor.state.wrapping_add(1);
        }
    } else {
        actor.position.move_x(-1);
        if world.probe_left_wall(actor) {
            actor.state = actor.state.wrapping_sub(1);
        }
    }
    if leg >= 2 {
        return land(actor, world);
    }
    let step = step_bounce_speed(actor);
    actor.position.move_y(-i16::from(step));
    if actor.motion_phase == BOUNCE_CAP {
        actor.motion_phase = 0;
        actor.state = actor.state.wrapping_add(2);
    }
    KnockbackEffect::None
}

/// `$5819B`'s caller: fall one step and report a landing. A hit reaction of 2
/// resumes walking with the contact kind it had.
fn land<W: WalkerWorld>(actor: &mut Actor, world: &mut W) -> KnockbackEffect {
    let step = step_fall_speed(actor);
    actor.position.move_y(i16::from(step));
    if !world.probe_ground(actor) {
        return KnockbackEffect::None;
    }
    let resume = (actor.hit_reaction == 2).then_some(actor.contact_kind);
    KnockbackEffect::Landed { resume }
}

/// `$5819B`: after the re-derive, restore the contact kind and walk on in the
/// facing the flip shows (`$3CB1` gives 1/2, then advanced).
pub fn resume_walk(actor: &mut Actor, contact_kind: ContactKind) {
    actor.contact_kind = contact_kind;
    actor.state = if actor.sprite_flip.horizontal() { 3 } else { 2 };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct World {
        left_wall: bool,
        right_wall: bool,
        ground: bool,
    }

    impl WalkerWorld for World {
        fn probe_left_wall(&mut self, _actor: &Actor) -> bool {
            self.left_wall
        }
        fn probe_right_wall(&mut self, _actor: &Actor) -> bool {
            self.right_wall
        }
        fn probe_ground(&mut self, _actor: &Actor) -> bool {
            self.ground
        }
        fn step_fall(&mut self, actor: &mut Actor) -> u8 {
            step_fall_speed(actor)
        }
    }

    #[test]
    fn stun_lies_still_then_spins_back_as_definition_one() {
        let mut actor = Actor::default();
        let mut world = World {
            ground: true,
            ..World::default()
        };
        assert_eq!(
            update_stun(&mut actor, FrameTick::new(0), &mut world),
            StunEffect::None
        );
        assert_eq!(
            (actor.state, actor.timer, actor.contact_kind.raw()),
            (1, 192, 6)
        );
        for _ in 0..192 {
            update_stun(&mut actor, FrameTick::new(1), &mut world);
        }
        assert_eq!(actor.state, 2);
        actor.sprite = 0x40;
        let mut revived = 0;
        for frame in 0..32 {
            if update_stun(&mut actor, FrameTick::new(frame), &mut world) == StunEffect::Revive {
                revived += 1;
            }
        }
        assert_eq!((revived, actor.definition, actor.sprite), (1, 1, 0x40));
    }

    #[test]
    fn airborne_stun_falls_on_the_fall_curve() {
        let mut actor = Actor::default();
        actor.state = 1;
        actor.timer = 10;
        update_stun(&mut actor, FrameTick::new(0), &mut World::default());
        assert_eq!(
            (actor.position.y, actor.motion_phase),
            (u16::from(FALL_STEP0), 1)
        );
    }

    const FALL_STEP0: u8 = super::super::FALL_SPEED[0];

    #[test]
    fn defeat_fall_removes_once_the_previous_projection_is_off_screen() {
        let mut actor = Actor::default();
        actor.state = 2;
        actor.screen.y = 0x9F;
        assert_eq!(update_defeat(&mut actor, None).unwrap(), DefeatEffect::None);
        assert_eq!(actor.position.y, u16::from(FALL_STEP0) * 2);
        actor.screen.y = 0xA0;
        assert_eq!(update_defeat(&mut actor, None).unwrap(), DefeatEffect::Remove);
    }

    #[test]
    fn knockback_bounces_then_falls_and_resumes_walking() {
        let mut actor = Actor::default();
        actor.hit_reaction = 2;
        actor.position.x = 0x80;
        actor.contact_kind = ContactKind::from_raw(0x4E);
        let mut world = World::default();
        assert_eq!(
            update_knockback(&mut actor, 0x40, &mut world),
            KnockbackEffect::Faced(Ordering::Less)
        );
        assert_eq!((actor.state, actor.sprite_flip.horizontal()), (1, true));
        for _ in 0..BOUNCE_CAP {
            assert_eq!(
                update_knockback(&mut actor, 0x40, &mut world),
                KnockbackEffect::None
            );
        }
        assert_eq!(
            (actor.state, actor.motion_phase, actor.position.x),
            (3, 0, 0x80 + 15)
        );
        world.ground = true;
        assert_eq!(
            update_knockback(&mut actor, 0x40, &mut world),
            KnockbackEffect::Landed {
                resume: Some(ContactKind::from_raw(0x4E))
            }
        );
        resume_walk(&mut actor, ContactKind::from_raw(0x4E));
        assert_eq!(actor.state, 3);
    }

    #[test]
    fn knockback_wall_flips_the_leg() {
        let mut actor = Actor::default();
        actor.state = 2;
        let mut world = World {
            left_wall: true,
            ..World::default()
        };
        update_knockback(&mut actor, 0, &mut world);
        assert_eq!(actor.state, 1);
    }
}
