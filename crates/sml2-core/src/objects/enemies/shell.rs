//! Native carried and kicked shell, bank 2: the carried pose (anim 4, `$58444`),
//! the release placement (`$8611`'s B-not-held tail) and the kicked walk
//! (anim 3, `$5839B`). Slot/world mapping lives in the `shell` adapter;
//! defeat, removal and the sprite re-derive stay engine work.
use std::cmp::Ordering;

use super::falling_dropper::step_fall_speed;
use super::SPIN_WOBBLE;
use crate::actor::{Actor, ContactKind, FrameTick};

/// A live kicked shell; the knock-out scan skips other shells by this kind.
pub const KICKED_CONTACT: ContactKind = ContactKind::from_raw(0x5F);
/// Contact kinds with this bit knock out what they overlap.
const KNOCKS_OUT: u8 = 0x40;
/// Pixels the kicked shell travels per update.
const KICK_SPEED: i16 = 3;
/// The release places the shell this far out of Mario.
const RELEASE_NUDGE: i16 = 16;

pub trait ShellWorld {
    fn probe_left_wall_low(&mut self, actor: &Actor) -> bool;
    fn probe_right_wall_low(&mut self, actor: &Actor) -> bool;
    fn probe_ground(&mut self, actor: &Actor) -> bool;
    /// `$3B68` eight pixels above the shell.
    fn probe_ceiling(&mut self, actor: &Actor) -> bool;
    /// `$5B697`: mark every enemy the shell overlaps for defeat; returns
    /// whether this update's hit flag is up.
    fn knock_out_overlaps(&mut self, actor: &Actor) -> bool;
}

/// Where Mario is and what he holds, for the carried pose.
#[derive(Clone, Copy)]
pub struct Carrier {
    pub x: u16,
    pub y: u16,
    pub big: bool,
    pub ducking: bool,
    pub travelling: bool,
    pub b_held: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CarriedEffect {
    None,
    /// Pipe travel drops the shell: clear the carry and remove the slot.
    RemoveForPipe,
    /// It hit an enemy or a ceiling: clear the carry and defeat it.
    Defeat,
    /// B released: the definition already stepped back to the kicked shell;
    /// the host re-derives the sprite, then runs [`place_kick`].
    Released,
}

/// `$5B607`: every fourth frame wobble the sprite, flipping on the last step.
pub fn animate_spin_wobble(actor: &mut Actor, frame: FrameTick) {
    if !frame.every(4) {
        return;
    }
    actor.animation_frame = actor.animation_frame.wrapping_add(1);
    if actor.animation_frame == 4 {
        actor.animation_frame = 0;
        actor.sprite_flip.set_horizontal(false);
    }
    actor.sprite = actor
        .sprite
        .wrapping_add(SPIN_WOBBLE[usize::from(actor.animation_frame)]);
    if actor.animation_frame == 3 {
        actor.sprite_flip.set_horizontal(true);
    }
}

/// `$5B4C1`: hold the shell in front of Mario, lower for small or ducking Mario.
fn follow(actor: &mut Actor, carrier: Carrier) {
    let lift: u8 = match (carrier.big, carrier.ducking) {
        (true, false) => 5,
        (false, true) => 20,
        _ => 14,
    };
    actor.position.x = carrier.x.wrapping_add(8);
    actor.position.y = carrier.y.wrapping_add(u16::from(lift));
}

/// Anim 4, `$58444`.
pub fn update_carried<W: ShellWorld>(
    actor: &mut Actor,
    carrier: Carrier,
    world: &mut W,
) -> CarriedEffect {
    if carrier.travelling {
        return CarriedEffect::RemoveForPipe;
    }
    if actor.state == 0 {
        actor.contact_kind = ContactKind::NONE;
        actor.state = 1;
        return CarriedEffect::None;
    }
    follow(actor, carrier);
    if world.knock_out_overlaps(actor) || world.probe_ceiling(actor) {
        return CarriedEffect::Defeat;
    }
    if carrier.b_held {
        return CarriedEffect::None;
    }
    actor.definition = actor.definition.wrapping_sub(1);
    CarriedEffect::Released
}

/// The release placement: step the shell 16 px out the way Mario faces; if a
/// low wall is there, step back and kick it the other way at once.
pub fn place_kick<W: ShellWorld>(actor: &mut Actor, mario_facing_right: bool, world: &mut W) {
    if mario_facing_right {
        actor.position.move_x(RELEASE_NUDGE);
        if !world.probe_right_wall_low(actor) {
            return;
        }
        actor.position.move_x(-RELEASE_NUDGE);
        actor.state = 1;
    } else {
        actor.position.move_x(-RELEASE_NUDGE);
        if !world.probe_left_wall_low(actor) {
            return;
        }
        actor.position.move_x(RELEASE_NUDGE);
        actor.state = 2;
    }
    actor.contact_kind = KICKED_CONTACT;
}

#[derive(Debug, PartialEq, Eq)]
pub enum KickedEffect {
    None,
    /// The kick started away from Mario; the ordering is his side.
    Faced(Ordering),
}

/// Anim 3, `$5839B`: states 1/2 slide right/left, 3/4 fall right/left. The
/// probes are the low wall pair (`$3BA4`/`$3C08`), not the walker's.
pub fn update_kicked<W: ShellWorld>(
    actor: &mut Actor,
    frame: FrameTick,
    mario_ref_x: u16,
    world: &mut W,
) -> KickedEffect {
    if actor.contact_kind.raw() & KNOCKS_OUT != 0 {
        world.knock_out_overlaps(actor);
    }
    animate_spin_wobble(actor, frame);
    let rightward = match actor.state {
        0 => {
            let side = actor.face_toward(mario_ref_x);
            actor.contact_kind = KICKED_CONTACT;
            return KickedEffect::Faced(side);
        }
        1 | 3 => true,
        2 | 4 => false,
        _ => return KickedEffect::None,
    };
    let blocked = if rightward {
        world.probe_right_wall_low(actor)
    } else {
        world.probe_left_wall_low(actor)
    };
    if blocked {
        actor.state = if rightward {
            actor.state.wrapping_add(1)
        } else {
            actor.state.wrapping_sub(1)
        };
        return KickedEffect::None;
    }
    actor
        .position
        .move_x(if rightward { KICK_SPEED } else { -KICK_SPEED });
    if actor.state <= 2 {
        // `$583CB`: no ground underfoot starts the fall.
        if !world.probe_ground(actor) {
            actor.state = actor.state.wrapping_add(2);
        }
    } else {
        fall(actor, world);
    }
    KickedEffect::None
}

/// `$58402`'s fall: land snapped to the tile, or drop one step faster than
/// the plain fall curve.
fn fall<W: ShellWorld>(actor: &mut Actor, world: &mut W) {
    if world.probe_ground(actor) {
        actor.motion_phase = 0;
        actor.state = actor.state.wrapping_sub(2);
        actor.position.y &= !0x0F;
        return;
    }
    let step = step_fall_speed(actor).wrapping_add(1);
    actor.position.move_y(i16::from(step));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct World {
        left_wall: bool,
        right_wall: bool,
        ground: bool,
        ceiling: bool,
        hit: bool,
        scans: u8,
    }

    impl ShellWorld for World {
        fn probe_left_wall_low(&mut self, _actor: &Actor) -> bool {
            self.left_wall
        }
        fn probe_right_wall_low(&mut self, _actor: &Actor) -> bool {
            self.right_wall
        }
        fn probe_ground(&mut self, _actor: &Actor) -> bool {
            self.ground
        }
        fn probe_ceiling(&mut self, _actor: &Actor) -> bool {
            self.ceiling
        }
        fn knock_out_overlaps(&mut self, _actor: &Actor) -> bool {
            self.scans += 1;
            self.hit
        }
    }

    fn carrier() -> Carrier {
        Carrier {
            x: 100,
            y: 200,
            big: false,
            ducking: false,
            travelling: false,
            b_held: true,
        }
    }

    #[test]
    fn carried_shell_rides_in_front_of_mario_until_released() {
        let mut actor = Actor::default();
        actor.state = 1;
        actor.definition = 4;
        let mut world = World::default();
        assert_eq!(
            update_carried(&mut actor, carrier(), &mut world),
            CarriedEffect::None
        );
        assert_eq!((actor.position.x, actor.position.y), (108, 214));
        let ducking = Carrier {
            ducking: true,
            ..carrier()
        };
        update_carried(&mut actor, ducking, &mut world);
        assert_eq!(actor.position.y, 220);
        let released = Carrier {
            b_held: false,
            ..carrier()
        };
        assert_eq!(
            update_carried(&mut actor, released, &mut world),
            CarriedEffect::Released
        );
        assert_eq!(actor.definition, 3);
    }

    #[test]
    fn carried_shell_is_defeated_by_a_ceiling() {
        let mut actor = Actor::default();
        actor.state = 1;
        let mut world = World {
            ceiling: true,
            ..World::default()
        };
        assert_eq!(
            update_carried(&mut actor, carrier(), &mut world),
            CarriedEffect::Defeat
        );
    }

    #[test]
    fn release_against_a_wall_kicks_back_the_other_way() {
        let mut actor = Actor::default();
        actor.position.x = 0x80;
        let mut world = World {
            right_wall: true,
            ..World::default()
        };
        place_kick(&mut actor, true, &mut world);
        assert_eq!(
            (actor.position.x, actor.state, actor.contact_kind),
            (0x80, 1, KICKED_CONTACT)
        );
    }

    #[test]
    fn kicked_shell_slides_falls_and_lands_snapped() {
        let mut actor = Actor::default();
        actor.position.x = 0x80;
        actor.position.y = 0x67;
        let mut world = World {
            ground: true,
            ..World::default()
        };
        assert_eq!(
            update_kicked(&mut actor, FrameTick(1), 0x40, &mut world),
            KickedEffect::Faced(Ordering::Less)
        );
        assert_eq!((actor.state, actor.contact_kind), (1, KICKED_CONTACT));
        update_kicked(&mut actor, FrameTick(1), 0x40, &mut world);
        assert_eq!((actor.position.x, world.scans), (0x83, 1));
        world.ground = false;
        update_kicked(&mut actor, FrameTick(1), 0x40, &mut world);
        assert_eq!(actor.state, 3);
        update_kicked(&mut actor, FrameTick(1), 0x40, &mut world);
        assert_eq!(actor.motion_phase, 1);
        world.ground = true;
        update_kicked(&mut actor, FrameTick(1), 0x40, &mut world);
        assert_eq!(
            (actor.state, actor.motion_phase, actor.position.y & 0x0F),
            (1, 0, 0)
        );
    }

    #[test]
    fn spin_wobble_flips_on_its_last_step() {
        let mut actor = Actor::default();
        actor.sprite = 0x10;
        for _ in 0..3 {
            animate_spin_wobble(&mut actor, FrameTick(0));
        }
        assert_eq!(
            (actor.animation_frame, actor.sprite_flip.horizontal()),
            (3, true)
        );
        animate_spin_wobble(&mut actor, FrameTick(0));
        assert_eq!(
            (
                actor.animation_frame,
                actor.sprite_flip.horizontal(),
                actor.sprite
            ),
            (0, false, 0x10)
        );
    }
}
