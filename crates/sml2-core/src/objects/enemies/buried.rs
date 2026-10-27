//! Buried enemy: wake nearby, emerge, then alternate walking and falling.
use crate::actor::{Actor, ContactKind, FrameTick, MotionProfileError, Position};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState,
    Motion(MotionProfileError),
}
impl From<MotionProfileError> for Error {
    fn from(error: MotionProfileError) -> Self {
        Self::Motion(error)
    }
}

pub(super) trait World {
    fn player_x(&self) -> u16;
    fn face_player(&mut self, position: Position);
    fn wall_blocked(&mut self, position: Position, right: bool) -> bool;
    fn grounded(&mut self, position: Position) -> bool;
    fn fall(&self) -> &[u8];
    fn request_walk_sound(&mut self);
    fn carry_player_up(&mut self, distance: u8);
}

pub(super) fn update(
    actor: &mut Actor,
    frame: FrameTick,
    world: &mut impl World,
) -> Result<(), Error> {
    match actor.state {
        0 => {
            actor.contact_kind = ContactKind::from_raw(32);
            let distance = (world.player_x() as u8).wrapping_sub(actor.position.x as u8) as i8;
            if distance.unsigned_abs() < 64 {
                actor.state = 5;
            }
        }
        1..=4 => {
            let walking = actor.state <= 2;
            if walking {
                actor.animate_two_frame_toggle(frame, 2);
            }
            if frame.every(2) {
                let right = actor.state == 2 || actor.state == 4;
                // Source checks the wall after committing the horizontal step.
                actor.position.move_x(if right { 1 } else { -1 });
                if world.wall_blocked(actor.position, right) {
                    world.request_walk_sound();
                }
            } else if walking {
                if !world.grounded(actor.position) {
                    actor.state += 2;
                }
            } else if world.grounded(actor.position) {
                actor.motion_phase = 0;
                actor.state -= 2;
            } else {
                let step = actor.motion_step(world.fall(), 21)?;
                actor.position.move_y(i16::from(step));
            }
        }
        5 => {
            actor.animate_two_frame_toggle(frame, 2);
            actor.animation_frame = actor.animation_frame.wrapping_add(1);
            if actor.animation_frame == 32 {
                actor.animation_frame = 0;
                actor.state = 6;
            }
        }
        6 => {
            if frame.every(8) {
                actor.sprite = actor.sprite.wrapping_add(1);
                if actor.sprite == 62 {
                    world.face_player(actor.position);
                    actor.face_toward(world.player_x());
                    actor.contact_kind = ContactKind::from_raw(77);
                    if actor.interactions.stood_on() {
                        // $5B3E0 leaves this coordinate component as the carry amount.
                        world.carry_player_up((actor.position.x >> 8) as u8);
                    }
                }
            }
        }
        _ => return Err(Error::InvalidState),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Scene {
        player: u16,
        wall: bool,
        floor: bool,
        probes: Vec<(Position, bool)>,
        sounds: u8,
        carries: Vec<u8>,
        faces: usize,
    }
    impl World for Scene {
        fn player_x(&self) -> u16 {
            self.player
        }
        fn face_player(&mut self, _: Position) {
            self.faces += 1;
        }
        fn wall_blocked(&mut self, p: Position, right: bool) -> bool {
            self.probes.push((p, right));
            self.wall
        }
        fn grounded(&mut self, _: Position) -> bool {
            self.floor
        }
        fn fall(&self) -> &[u8] {
            &super::super::FALL_SPEED
        }
        fn request_walk_sound(&mut self) {
            self.sounds += 1;
        }
        fn carry_player_up(&mut self, d: u8) {
            self.carries.push(d);
        }
    }
    #[test]
    fn handler_0x589a6_wake_signed_low_byte_boundaries() {
        for (distance, expected) in [
            (0, 5),
            (63, 5),
            (64, 0),
            (127, 0),
            (128, 0),
            (192, 0),
            (193, 5),
            (255, 5),
        ] {
            let mut a = Actor::default();
            a.position.x = 0x12ff;
            let mut w = Scene {
                player: 0x3400 | 255u8.wrapping_add(distance) as u16,
                ..Scene::default()
            };
            update(&mut a, FrameTick::new(0), &mut w).unwrap();
            assert_eq!((a.state, a.contact_kind.raw()), (expected, 32));
        }
    }
    #[test]
    fn handler_0x589a6_horizontal_gates_commit_before_wall_and_sound() {
        for state in 1..=4 {
            for blocked in [false, true] {
                let mut a = Actor::default();
                a.state = state;
                a.sprite = 60;
                a.position.x = if state % 2 == 0 { 65535 } else { 0 };
                let mut w = Scene {
                    wall: blocked,
                    ..Scene::default()
                };
                update(&mut a, FrameTick::new(0), &mut w).unwrap();
                assert_eq!(a.position.x, if state % 2 == 0 { 0 } else { 65535 });
                assert_eq!(a.sprite, if state <= 2 { 61 } else { 60 });
                assert_eq!(w.probes, vec![(a.position, state % 2 == 0)]);
                assert_eq!(w.sounds, u8::from(blocked));
                assert_eq!(a.state, state);
            }
        }
    }
    #[test]
    fn handler_0x589a6_fall_entry_landing_and_profile_cap() {
        let mut a = Actor::default();
        a.state = 1;
        a.motion_phase = 20;
        a.position.y = 65535;
        let mut w = Scene::default();
        update(&mut a, FrameTick::new(1), &mut w).unwrap();
        assert_eq!((a.state, a.position.y, a.motion_phase), (3, 65535, 20));
        update(&mut a, FrameTick::new(3), &mut w).unwrap();
        assert_eq!((a.position.y, a.motion_phase), (2, 21));
        update(&mut a, FrameTick::new(5), &mut w).unwrap();
        assert_eq!((a.position.y, a.motion_phase), (5, 21));
        w.floor = true;
        update(&mut a, FrameTick::new(7), &mut w).unwrap();
        assert_eq!((a.state, a.position.y, a.motion_phase), (1, 5, 0));
        a.state = 2;
        update(&mut a, FrameTick::new(9), &mut w).unwrap();
        assert_eq!(a.state, 2);
        a.state = 4;
        update(&mut a, FrameTick::new(11), &mut w).unwrap();
        assert_eq!(a.state, 2);
    }
    #[test]
    fn handler_0x589a6_wait_equality_and_emergence_cadences() {
        let mut a = Actor::default();
        a.state = 5;
        a.animation_frame = 30;
        a.sprite = 60;
        let mut w = Scene::default();
        update(&mut a, FrameTick::new(1), &mut w).unwrap();
        assert_eq!((a.state, a.animation_frame, a.sprite), (5, 31, 60));
        update(&mut a, FrameTick::new(2), &mut w).unwrap();
        assert_eq!((a.state, a.animation_frame, a.sprite), (6, 0, 61));
        update(&mut a, FrameTick::new(4), &mut w).unwrap();
        assert_eq!(a.sprite, 61);
        update(&mut a, FrameTick::new(7), &mut w).unwrap();
        assert_eq!(a.sprite, 61);
        update(&mut a, FrameTick::new(8), &mut w).unwrap();
        assert_eq!((a.state, a.sprite, a.contact_kind.raw()), (1, 62, 77));
        a.state = 5;
        a.animation_frame = 32;
        update(&mut a, FrameTick::new(9), &mut w).unwrap();
        assert_eq!((a.state, a.animation_frame), (5, 33));
        a.state = 5;
        a.animation_frame = 255;
        update(&mut a, FrameTick::new(9), &mut w).unwrap();
        assert_eq!((a.state, a.animation_frame), (5, 0));
        a.state = 6;
        a.sprite = 62;
        update(&mut a, FrameTick::new(16), &mut w).unwrap();
        assert_eq!((a.state, a.sprite), (6, 63));
        a.state = 6;
        a.sprite = 255;
        update(&mut a, FrameTick::new(16), &mut w).unwrap();
        assert_eq!((a.state, a.sprite), (6, 0));
    }
    #[test]
    fn handler_0x589a6_emergence_faces_and_carries_coordinate_high_byte() {
        use crate::actor::{InteractionFlags, SpriteFlipFlags};
        for high in [0, 1, 2, 255] {
            for stood in [false, true] {
                for relation in [-1i16, 0, 1] {
                    let mut a = Actor::default();
                    a.state = 6;
                    a.sprite = 61;
                    a.position.x = (high as u16) << 8;
                    a.sprite_flip = SpriteFlipFlags::from_bits(255);
                    a.interactions = InteractionFlags::from_bits(if stood { 8 } else { 0 });
                    let target = a.position.x.wrapping_add_signed(relation);
                    let mut w = Scene {
                        player: target,
                        ..Scene::default()
                    };
                    update(&mut a, FrameTick::new(8), &mut w).unwrap();
                    let right = target > a.position.x;
                    assert_eq!(
                        (a.state, a.sprite_flip.bits(), w.faces),
                        (if right { 2 } else { 1 }, if right { 32 } else { 0 }, 1)
                    );
                    assert_eq!(w.carries, if stood { vec![high] } else { vec![] });
                }
            }
        }
    }
}
