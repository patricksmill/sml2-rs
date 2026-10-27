//! DX home `$16E9/$16F2`, bank1 H `$42E4` and normal V `$44E4`.
use super::*;
use crate::character::{Character, MovementCurves};
use crate::level::{LevelLayout, LAYOUT_SIZE};
use std::sync::Arc;

fn curves() -> Arc<MovementCurves> {
    let mut h = SPEED_H;
    h[0xF7] = 2;
    h[0xF8] = 3;
    let mut v: [u8; 76] = SPEED_V[..76].try_into().unwrap();
    v[59..=72].fill(0xFC);
    Arc::new(MovementCurves {
        horizontal: h, normal: v,
        swim: SPEED_V_GOO[..76].try_into().unwrap(),
        moon: SPEED_V_WATER[..76].try_into().unwrap(),
    })
}

#[test]
fn dx_friction_keeps_neutral_snap_gates_and_held_input_phase() {
    for character in [Character::Mario, Character::Luigi] {
        for (hvel, right, swim, moon, grounded, expected_mario, expected_luigi) in [
            (0x90, false, false, 0, true, 0x8C, 0x8D),
            (0x70, false, false, 0, true, 0x74, 0x73),
            (0x90, true, false, 0, true, 0x94, 0x95),
            (0x90, false, false, 0, false, 0x90, 0x90),
            (0x90, false, true, 0, false, 0x8C, 0x8D),
            (0x90, false, false, 2, false, 0x8C, 0x8D),
            (0x80, false, false, 0, true, 0x80, 0x80),
            (0x81, false, false, 0, true, 0x80, 0x80),
            (0x82, false, false, 0, true, 0x80, 0x80),
            (0x83, false, false, 0, true, 0x80, 0x80),
        ] {
            let layout = LevelLayout::from_bytes(&vec![if swim { 0x4C } else if moon == 2 { 0x52 } else { 0x60 }; LAYOUT_SIZE]).unwrap();
            let mut sim = PlayerSim::new();
            sim.on_ground = grounded; sim.on_ground_obj = grounded; sim.vvel = if grounded {0} else {24};
            sim.camx = 100; sim.x = 100; sim.y = 200; sim.hvel = hvel; sim.swim = swim; sim.moon = moon;
            sim.set_dx_movement(character, Some(curves()));
            sim.step(&PlayerKeys {right, ..Default::default()}, false, false, &layout);
            assert_eq!(sim.hvel, if character == Character::Mario {expected_mario} else {expected_luigi}, "{character:?} {hvel:02X} held={right} swim={swim} moon={moon}");
        }
    }
}

#[test]
fn dx_displacements_use_wrapped_phase_and_normal_swim_moon_precedence() {
    for (moon, swim, delta) in [(0, false, -4i16), (0, true, -1), (1, true, -1)] {
        let layout = LevelLayout::from_bytes(&vec![if swim {0x4C} else {0x60}; LAYOUT_SIZE]).unwrap();
        let mut sim = PlayerSim::new();
        sim.on_ground = false; sim.no_jump_cut = true;
        sim.camx = 100; sim.x = 100; sim.y = 200; sim.vvel = 60; sim.moon = moon; sim.swim = swim;
        sim.set_dx_movement(Character::Luigi, Some(curves()));
        sim.step(&PlayerKeys {a: true, ..Default::default()}, false, false, &layout);
        assert_eq!((sim.vvel, sim.y), (if moon == 1 {63} else {59}, (200i16 + delta) as u16));
    }
    let layout = LevelLayout::from_bytes(&[0x60; LAYOUT_SIZE]).unwrap();
    for (hvel, phase, displacement) in [(0xF7, 0, 2), (0xF8, 0, 3), (0xF6, 1, 2), (0xFE, 1, 1)] {
        let mut sim = PlayerSim::new();
        sim.on_ground = false;
        sim.camx = 100; sim.x = 100; sim.y = 200; sim.hvel = hvel; sim.phase = phase;
        sim.set_dx_movement(Character::Luigi, Some(curves()));
        sim.step(&PlayerKeys::default(), false, false, &layout);
        assert_eq!(sim.x, 100 + displacement, "hvel={hvel:02X} phase={phase}");
    }
}
