//! Hopping enemy with a six-tick turning animation (ROM $58848).
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

/// Persistent states from the source dispatch table. Unknown bytes remain errors.
#[derive(Clone, Copy)]
#[repr(u8)]
enum HoppingState {
    Initialize = 0,
    HopLeft = 1,
    HopRight = 2,
    TurnLeft = 3,
    TurnRight = 4,
}

impl HoppingState {
    fn from_raw(value: u8) -> Result<Self, Error> {
        match value {
            0 => Ok(Self::Initialize),
            1 => Ok(Self::HopLeft),
            2 => Ok(Self::HopRight),
            3 => Ok(Self::TurnLeft),
            4 => Ok(Self::TurnRight),
            _ => Err(Error::InvalidState),
        }
    }
}

const MOTION_PHASE_CAP: u8 = 12;
const LANDING_PAUSE: u8 = 8;
const TURN_FRAME_PERIOD: u64 = 4;
const TURN_STEPS: u8 = 6;
const NEAR_PLAYER_DISTANCE: u8 = 48;
const RISING_SPRITE: u8 = 56;
const FALLING_SPRITE: u8 = 52;

pub(super) trait World {
    fn player_x(&self) -> u16;
    fn face_player(&mut self, position: Position) -> bool;
    fn wall_blocked(&mut self, position: Position, right: bool) -> bool;
    fn grounded(&mut self, position: Position) -> bool;
    fn rise(&self) -> &[u8];
    fn fall(&self) -> &[u8];
}

fn reverse(actor: &mut Actor) {
    actor.set_motion_counter(0);
    actor.sprite_flip.toggle_horizontal();
    actor.state ^= 3;
}

pub(super) fn update(
    actor: &mut Actor,
    frame: FrameTick,
    world: &mut impl World,
) -> Result<(), Error> {
    match HoppingState::from_raw(actor.state)? {
        HoppingState::Initialize => {
            let _ = world.face_player(actor.position);
            actor.face_toward(world.player_x());
            actor.contact_kind = ContactKind::from_raw(77);
            actor.timer = 1;
        }
        HoppingState::HopLeft | HoppingState::HopRight => hop(actor, world)?,
        HoppingState::TurnLeft | HoppingState::TurnRight => turn(actor, frame),
    }
    Ok(())
}

fn hop(actor: &mut Actor, world: &mut impl World) -> Result<(), Error> {
    if actor.timer != 0 {
        actor.timer -= 1;
        if actor.timer != 0 {
            return Ok(());
        }
        actor.sprite = RISING_SPRITE;
    }
    let right = actor.state == HoppingState::HopRight as u8;
    if world.wall_blocked(actor.position, right) {
        reverse(actor);
        return Ok(());
    }
    actor.position.move_x(if right { 1 } else { -1 });
    if actor.motion_leg == 0 {
        let step = actor.motion_step(world.rise(), MOTION_PHASE_CAP)?;
        actor.position.move_y(-i16::from(step));
        if actor.motion_phase == MOTION_PHASE_CAP {
            actor.motion_phase = 0;
            actor.motion_leg = 1;
            actor.sprite = FALLING_SPRITE;
        }
        return Ok(());
    }
    if !world.grounded(actor.position) {
        let step = actor.motion_step(world.fall(), MOTION_PHASE_CAP)?;
        actor.position.move_y(i16::from(step));
        return Ok(());
    }
    actor.motion_phase = 0;
    actor.motion_leg = 0;
    actor.timer = LANDING_PAUSE;
    let distance = (world.player_x() as u8).wrapping_sub(actor.position.x as u8) as i8;
    if distance.unsigned_abs() >= NEAR_PLAYER_DISTANCE {
        actor.set_motion_counter(actor.motion_counter().wrapping_add(1));
        if actor.motion_counter() == 3 {
            reverse(actor);
        }
        return Ok(());
    }
    let face_left = world.face_player(actor.position);
    let desired_state = if face_left {
        HoppingState::HopLeft as u8
    } else {
        HoppingState::HopRight as u8
    };
    if actor.state == desired_state {
        actor.set_motion_counter(actor.motion_counter().wrapping_add(1));
        actor.state = actor.state.wrapping_add(2);
    } else {
        actor.state = desired_state + 2;
        actor.set_motion_counter(2u8.wrapping_sub(actor.motion_counter()));
        actor.sprite_flip.toggle_horizontal();
    }
    Ok(())
}

fn turn(actor: &mut Actor, frame: FrameTick) {
    if actor.timer != 0 {
        actor.timer -= 1;
        return;
    }
    if !frame.every(TURN_FRAME_PERIOD) {
        return;
    }
    if actor.animation_frame == TURN_STEPS {
        actor.animation_frame = 0;
        actor.timer = LANDING_PAUSE;
        actor.state = actor.state.wrapping_sub(2);
        if actor.motion_counter() == 3 {
            reverse(actor);
        }
        return;
    }
    let first_half = actor.animation_frame < TURN_STEPS / 2;
    actor.animation_frame = actor.animation_frame.wrapping_add(1);
    actor.sprite = if first_half {
        actor.sprite.wrapping_add(1)
    } else {
        actor.sprite.wrapping_sub(1)
    };
    let left = (actor.state == HoppingState::TurnLeft as u8) == first_half;
    actor.position.move_x(if left { -4 } else { 4 });
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Scene {
        player: u16,
        wall: bool,
        floor: bool,
        faced: usize,
    }
    impl World for Scene {
        fn player_x(&self) -> u16 {
            self.player
        }
        fn face_player(&mut self, p: Position) -> bool {
            self.faced += 1;
            self.player < p.x || (self.player == p.x && self.player as u8 & 0x80 != 0)
        }
        fn wall_blocked(&mut self, _: Position, _: bool) -> bool {
            self.wall
        }
        fn grounded(&mut self, _: Position) -> bool {
            self.floor
        }
        fn rise(&self) -> &[u8] {
            &[4; 256]
        }
        fn fall(&self) -> &[u8] {
            &[2; 256]
        }
    }
    fn scene(player: u16) -> Scene {
        Scene {
            player,
            wall: false,
            floor: false,
            faced: 0,
        }
    }

    #[test]
    fn handler_0x58848_init_timer_and_coordinate_boundaries() {
        let mut actor = Actor::default();
        let mut world = scene(1);
        update(&mut actor, FrameTick(0), &mut world).unwrap();
        assert_eq!(
            (actor.state, actor.contact_kind.raw(), actor.timer),
            (2, 77, 1)
        );
        actor.position.x = 0xffff;
        actor.position.y = 1;
        update(&mut actor, FrameTick(1), &mut world).unwrap();
        assert_eq!(
            (
                actor.position.x,
                actor.position.y,
                actor.sprite,
                actor.timer
            ),
            (0, 0xfffd, 56, 0)
        );
        actor.timer = 2;
        let before = actor.position;
        update(&mut actor, FrameTick(2), &mut world).unwrap();
        assert_eq!((actor.position, actor.timer), (before, 1));
    }

    #[test]
    fn handler_0x58848_hop_apex_wall_and_landing_boundaries() {
        let mut actor = Actor::default();
        actor.state = 1;
        actor.motion_phase = 11;
        let mut world = scene(200);
        update(&mut actor, FrameTick(0), &mut world).unwrap();
        assert_eq!(
            (actor.motion_phase, actor.motion_leg, actor.sprite),
            (0, 1, 52)
        );
        update(&mut actor, FrameTick(1), &mut world).unwrap();
        assert_eq!(actor.motion_phase, 1);
        world.wall = true;
        actor.set_motion_counter(2);
        let before = actor.position;
        update(&mut actor, FrameTick(2), &mut world).unwrap();
        assert_eq!(
            (actor.position, actor.state, actor.motion_counter()),
            (before, 2, 0)
        );
        world.wall = false;
        world.floor = true;
        actor.position.x = 152;
        actor.set_motion_counter(2);
        // Movement happens before distance: 200 - 153 = 47 selects turning.
        update(&mut actor, FrameTick(3), &mut world).unwrap();
        assert_eq!(
            (actor.state, actor.motion_counter(), actor.timer),
            (4, 3, 8)
        );
        actor.state = 2;
        actor.position.x = 151;
        actor.motion_leg = 1;
        actor.timer = 0;
        actor.set_motion_counter(2);
        update(&mut actor, FrameTick(4), &mut world).unwrap();
        assert_eq!((actor.state, actor.motion_counter()), (1, 0));
    }

    #[test]
    fn handler_0x58848_turn_frame_gate_delay_and_full_cycle() {
        for state in [3, 4] {
            let mut actor = Actor::default();
            actor.state = state;
            actor.sprite = 56;
            actor.timer = 1;
            let mut world = scene(0);
            update(&mut actor, FrameTick(0), &mut world).unwrap();
            assert_eq!((actor.timer, actor.animation_frame), (0, 0));
            update(&mut actor, FrameTick(1), &mut world).unwrap();
            assert_eq!(actor.animation_frame, 0);
            for step in 1..=6 {
                update(&mut actor, FrameTick(step * 4), &mut world).unwrap();
                assert_eq!(actor.animation_frame, step as u8);
                let excursion = if step <= 3 { step * 4 } else { (6 - step) * 4 };
                assert_eq!(
                    actor.position.x,
                    if state == 3 {
                        0u16.wrapping_sub(excursion as u16)
                    } else {
                        excursion as u16
                    }
                );
                let delta = if step <= 3 { step } else { 6 - step };
                assert_eq!(actor.sprite, 56 + delta as u8);
            }
            assert_eq!(actor.position.x, 0);
            actor.set_motion_counter(3);
            update(&mut actor, FrameTick(28), &mut world).unwrap();
            assert_eq!(
                (
                    actor.state,
                    actor.timer,
                    actor.animation_frame,
                    actor.motion_counter()
                ),
                ((state - 2) ^ 3, 8, 0, 0)
            );
        }
    }

    #[test]
    fn handler_0x58848_near_player_turn_reverses_and_equal_position_uses_low_bit_seven() {
        let mut actor = Actor::default();
        actor.state = 2;
        actor.motion_leg = 1;
        actor.position.x = 128;
        actor.set_motion_counter(1);
        let mut world = scene(129);
        world.floor = true;
        update(&mut actor, FrameTick(0), &mut world).unwrap();
        assert_eq!(
            (
                actor.state,
                actor.motion_counter(),
                actor.sprite_flip.bits()
            ),
            (3, 1, 32)
        );
        assert_eq!(world.faced, 1);
    }
}
