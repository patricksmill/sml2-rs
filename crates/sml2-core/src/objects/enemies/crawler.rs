//! Native crawler, bank $16: anim 8 (`$5862E`) and the shared crawl body
//! (`$58642`) that the turning, blinking and rising crawlers enter. On odd
//! frames it crawls one pixel; on even ones it hovers up, falls back down and,
//! after four hops, turns to face Mario again. Slot/world mapping lives in the
//! `walkers` adapter.
use std::cmp::Ordering;

use super::falling_dropper::step_fall_speed;
use super::walker::WalkerWorld;
use super::HOVER_SPEED;
use crate::actor::{Actor, ContactKind, FlashFlags, FrameTick};

/// The hover rises along this curve, then falls.
const HOVER_CAP: u8 = 21;
/// Hops before the crawler faces Mario afresh.
const HOPS_PER_CYCLE: u8 = 4;
/// A landing restarts the hover partway up its curve.
const LANDED_HOVER_PHASE: u8 = 10;
/// `motion_leg` while falling back down.
const FALLING: u8 = 1;

pub trait CrawlerWorld: WalkerWorld {
    /// `$3B68` eighteen pixels above the crawler.
    fn probe_ceiling(&mut self, actor: &Actor) -> bool;
}

/// Anim 8, `$5862E`: blink every other frame, face Mario, then crawl. Returns
/// Mario's side whenever the crawler faced him.
pub fn update<W: CrawlerWorld>(
    actor: &mut Actor,
    frame: FrameTick,
    mario_ref_x: u16,
    world: &mut W,
) -> Option<Ordering> {
    actor.animate_two_frame_toggle(frame, 2);
    if actor.state == 0 {
        let side = actor.face_toward(mario_ref_x);
        actor.flash = FlashFlags::from_bits(16);
        actor.contact_kind = ContactKind::from_raw(0x56);
        return Some(side);
    }
    crawl(actor, frame, mario_ref_x, world)
}

/// `$58642`: states 1/3 crawl left, 2/4 right; a wall turns the crawler back
/// to state 1 or 2.
pub fn crawl<W: CrawlerWorld>(
    actor: &mut Actor,
    frame: FrameTick,
    mario_ref_x: u16,
    world: &mut W,
) -> Option<Ordering> {
    let leftward = match actor.state {
        1 | 3 => true,
        2 | 4 => false,
        _ => return None,
    };
    if !frame.every(2) {
        return hop(actor, mario_ref_x, world);
    }
    let blocked = if leftward {
        actor.position.move_x(-1);
        world.probe_left_wall(actor)
    } else {
        actor.position.move_x(1);
        world.probe_right_wall(actor)
    };
    if blocked {
        actor.state = if leftward { 2 } else { 1 };
        actor.sprite_flip.toggle_horizontal();
    }
    None
}

/// Hover-curve step clamped at its cap.
fn step_hover_speed(actor: &mut Actor) -> u8 {
    let speed = HOVER_SPEED[usize::from(actor.motion_phase.min(HOVER_CAP))];
    if actor.motion_phase != HOVER_CAP {
        actor.motion_phase = actor.motion_phase.wrapping_add(1);
    }
    speed
}

/// `$5868C` hovering up (until the curve ends or a ceiling), `$586AD` falling.
fn hop<W: CrawlerWorld>(actor: &mut Actor, mario_ref_x: u16, world: &mut W) -> Option<Ordering> {
    if actor.motion_leg != FALLING {
        let step = step_hover_speed(actor);
        if actor.motion_phase == HOVER_CAP {
            actor.motion_phase = 0;
            actor.motion_leg = FALLING;
            return None;
        }
        actor.position.move_y(-i16::from(step));
        if world.probe_ceiling(actor) {
            actor.motion_phase = 0;
            actor.motion_leg = FALLING;
        }
        return None;
    }
    let step = step_fall_speed(actor);
    actor.position.move_y(i16::from(step));
    if !world.probe_ground(actor) {
        return None;
    }
    actor.motion_leg = 0;
    let hops = actor.motion_counter().wrapping_add(1);
    actor.set_motion_counter(hops);
    if hops == HOPS_PER_CYCLE {
        // `$586D6`: restart the cycle facing Mario.
        actor.motion_phase = 0;
        actor.set_motion_counter(0);
        return Some(actor.face_toward(mario_ref_x));
    }
    actor.motion_phase = LANDED_HOVER_PHASE;
    if actor.state < 3 {
        actor.state += 2;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct World {
        wall: bool,
        ground: bool,
        ceiling: bool,
    }

    impl WalkerWorld for World {
        fn probe_left_wall(&mut self, _actor: &Actor) -> bool {
            self.wall
        }
        fn probe_right_wall(&mut self, _actor: &Actor) -> bool {
            self.wall
        }
        fn probe_ground(&mut self, _actor: &Actor) -> bool {
            self.ground
        }
        fn step_fall(&mut self, actor: &mut Actor) -> u8 {
            step_fall_speed(actor)
        }
    }

    impl CrawlerWorld for World {
        fn probe_ceiling(&mut self, _actor: &Actor) -> bool {
            self.ceiling
        }
    }

    #[test]
    fn crawler_faces_mario_then_crawls_on_even_frames() {
        let mut actor = Actor::default();
        actor.position.x = 0x80;
        let mut world = World::default();
        assert_eq!(
            update(&mut actor, FrameTick::new(0), 0x90, &mut world),
            Some(Ordering::Greater)
        );
        assert_eq!(
            (actor.state, actor.contact_kind.raw(), actor.flash.bits()),
            (2, 0x56, 16)
        );
        update(&mut actor, FrameTick::new(2), 0x90, &mut world);
        assert_eq!(actor.position.x, 0x81);
        world.wall = true;
        update(&mut actor, FrameTick::new(4), 0x90, &mut world);
        assert_eq!((actor.state, actor.sprite_flip.horizontal()), (1, false));
    }

    #[test]
    fn hover_rises_until_the_curve_ends_then_falls() {
        let mut actor = Actor::default();
        actor.state = 1;
        actor.position.y = 0x200;
        let mut world = World::default();
        for _ in 0..HOVER_CAP {
            crawl(&mut actor, FrameTick::new(1), 0, &mut world);
        }
        assert_eq!((actor.motion_leg, actor.motion_phase), (FALLING, 0));
        assert!(actor.position.y < 0x200);
    }

    #[test]
    fn a_ceiling_cuts_the_hover_short() {
        let mut actor = Actor::default();
        actor.state = 1;
        let mut world = World {
            ceiling: true,
            ..World::default()
        };
        crawl(&mut actor, FrameTick::new(1), 0, &mut world);
        assert_eq!((actor.motion_leg, actor.motion_phase), (FALLING, 0));
    }

    #[test]
    fn landings_count_hops_and_the_fourth_faces_mario() {
        let mut actor = Actor::default();
        actor.state = 1;
        actor.position.x = 0x80;
        let mut world = World {
            ground: true,
            ..World::default()
        };
        for hop in 1..4 {
            actor.motion_leg = FALLING;
            assert_eq!(crawl(&mut actor, FrameTick::new(1), 0x10, &mut world), None);
            assert_eq!(
                (actor.motion_counter(), actor.motion_phase, actor.state),
                (hop, 10, 3)
            );
        }
        actor.motion_leg = FALLING;
        assert_eq!(
            crawl(&mut actor, FrameTick::new(1), 0x10, &mut world),
            Some(Ordering::Less)
        );
        assert_eq!(
            (actor.motion_counter(), actor.motion_phase, actor.state),
            (0, 0, 1)
        );
    }
}
