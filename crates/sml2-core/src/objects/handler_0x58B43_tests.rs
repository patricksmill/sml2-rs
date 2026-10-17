use super::*;
use crate::object::ObjectTables;

fn fixture(prefix: u8, state: u8) -> (ObjectSim, Staging) {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.facing = 99;
    sim.move_flags = 0x97;
    let mut st = Staging::default();
    st.b = [prefix; 32];
    st.b[15] = state;
    st.b[16] = 2;
    st.b[26] = 0;
    st.b[28] = 0;
    st.px = 0x1234;
    st.py = 0x5678;
    (sim, st)
}
fn check(sim: &mut ObjectSim, st: &mut Staging, expected: [u8; 32], facing: u8) {
    sim.handler_0x58b43(st);
    assert_eq!(st.b, expected);
    assert_eq!(
        (st.px, st.py, sim.move_flags, sim.facing),
        (0x1234, 0x5678, 0x97, facing)
    );
    assert!(sim.unported.is_empty());
}
fn set_position(slot: &mut [u8; 32], x: u16, y: u16) {
    slot[0..2].copy_from_slice(&x.to_be_bytes());
    slot[3..5].copy_from_slice(&y.to_be_bytes());
}
#[test]
fn handler_0x58b43_init_full_x_relation_preserves_all_other_bytes() {
    for prefix in [0, 0xa5, 255] {
        for (x, target, state, flip, facing) in [
            (0x100, 0xff, 1, 0, 255),
            (0xff, 0x100, 2, 32, 1),
            (0xffff, 0xffff, 1, 0, 0),
            (0, 0, 1, 0, 0),
        ] {
            let (mut sim, mut st) = fixture(prefix, 0);
            st.b[0..2].copy_from_slice(&u16::to_be_bytes(x));
            st.b[16] = prefix;
            st.b[26] = prefix;
            st.b[28] = prefix;
            sim.mario_ref_x = target;
            let mut expected = st.b;
            expected[15] = state;
            expected[13] = flip;
            expected[18] = 77;
            check(&mut sim, &mut st, expected, facing);
        }
    }
}
#[test]
fn handler_0x58b43_timers_animation_and_expiry_double_step() {
    for prefix in [0, 0xa5, 255] {
        for (timer, frame, phase, sprite, next_timer, next_phase, delta, moves) in [
            (40, 0, 0, 255u8, 39, 1, 1i8, false),
            (40, 1, 0, 255, 39, 0, 0, false),
            (2, 0, 2, 0, 1, 0, -2, false),
            (1, 0, 0, 255, 0, 2, 2, true),
            (1, 0, 1, 0, 0, 0, -1, true),
            (1, 1, 0, 255, 0, 1, 1, false),
            (0, 1, 2, 0, 0, 0, -2, false),
            (0, 1, 255, 0, 0, 0, -2, false),
        ] {
            let (mut sim, mut st) = fixture(prefix, 2);
            sim.fff0 = frame;
            st.b[28] = timer;
            st.b[26] = phase;
            st.b[11] = sprite;
            st.b[24] = 0;
            set_position(&mut st.b, 0xffff, 0xffff);
            let mut expected = st.b;
            expected[28] = next_timer;
            expected[26] = next_phase;
            expected[11] = sprite.wrapping_add_signed(delta);
            if moves {
                set_position(&mut expected, 0, 0);
                expected[16] = 3;
            }
            check(&mut sim, &mut st, expected, 99);
        }
    }
}
#[test]
fn handler_0x58b43_motion_horizontal_samples_vertical_unit_and_wrap() {
    for state in [1, 2] {
        for leg in [0, 1, 255] {
            for phase in 0..20u8 {
                let (mut sim, mut st) = fixture(0xa5, state);
                st.b[16] = phase;
                st.b[24] = leg;
                let x = if state == 1 { 0u16 } else { 65535u16 };
                let y = if leg == 0 { 65535u16 } else { 0u16 };
                set_position(&mut st.b, x, y);
                let mut expected = st.b;
                let distance = u16::from(
                    [
                        0u8, 0, 1, 1, 1, 2, 1, 2, 2, 1, 2, 3, 2, 3, 3, 3, 2, 3, 3, 2, 3, 3,
                    ][usize::from(phase)],
                );
                set_position(
                    &mut expected,
                    if state == 1 {
                        x.wrapping_sub(distance)
                    } else {
                        x.wrapping_add(distance)
                    },
                    if leg == 0 { 0 } else { 65535 },
                );
                expected[16] += 1;
                expected[26] = 1;
                expected[11] = expected[11].wrapping_add(1);
                check(&mut sim, &mut st, expected, 99);
            }
        }
    }
}
#[test]
fn handler_0x58b43_retarget_full_y_equality_low_byte_and_global_overwrite() {
    for phase in [20, 21] {
        for state in [1, 2] {
            for y in [0u16, 1, 2, 255, 256, 257, 0xffff] {
                for relation in [-1i16, 0, 1] {
                    let (mut sim, mut st) = fixture(0xa5, state);
                    st.b[16] = phase;
                    st.b[24] = 0;
                    set_position(&mut st.b, 0x100, y.wrapping_sub(1));
                    sim.mario_ref_x = if state == 1 { 0x200 } else { 0 };
                    sim.mario_ref_y = y.wrapping_add_signed(relation);
                    let mut expected = st.b;
                    set_position(&mut expected, if state == 1 { 0xfd } else { 0x103 }, y);
                    expected[15] = if state == 1 { 2 } else { 1 };
                    expected[13] = if state == 1 { 32 } else { 0 };
                    expected[16] = 0;
                    expected[26] = 1;
                    expected[11] = 0xa6;
                    expected[28] = 40;
                    let ordering = sim.mario_ref_y.cmp(&y);
                    expected[24] = u8::from(
                        !(ordering == Ordering::Greater
                            || (ordering == Ordering::Equal && y as u8 == 1)),
                    );
                    let facing = match ordering {
                        Ordering::Less => 255,
                        Ordering::Equal => 0,
                        Ordering::Greater => 1,
                    };
                    check(&mut sim, &mut st, expected, facing);
                }
            }
        }
    }
}
#[test]
fn handler_0x58b43_complete_arc_and_pause_cadence() {
    let (mut sim, mut st) = fixture(0, 2);
    st.b[16] = 0;
    set_position(&mut st.b, 100, 100);
    sim.mario_ref_x = 1000;
    sim.mario_ref_y = 1000;
    for _ in 0..21 {
        sim.handler_0x58b43(&mut st);
    }
    let mut expected = [0; 32];
    set_position(&mut expected, 140, 121);
    expected[15] = 2;
    expected[13] = 32;
    expected[28] = 40;
    assert_eq!(st.b, expected);
    for frame in 1..40 {
        sim.fff0 = frame;
        sim.handler_0x58b43(&mut st);
    }
    expected[28] = 1;
    expected[26] = 1;
    expected[11] = 1;
    assert_eq!(st.b, expected);
    sim.fff0 = 40;
    sim.handler_0x58b43(&mut st);
    expected[28] = 0;
    expected[26] = 0;
    expected[11] = 0;
    expected[16] = 1;
    set_position(&mut expected, 140, 122);
    assert_eq!(st.b, expected);
    assert!(sim.unported.is_empty());
}
#[test]
fn handler_0x58b43_diagnostics_do_not_clamp_unknown_indices() {
    for (state, phase, anim, expected_anim, sprite_delta) in
        [(3, 2, 0, 0, 0u8), (1, 22, 0, 1, 1), (1, 2, 3, 4, 0)]
    {
        let (mut sim, mut st) = fixture(0xa5, state);
        st.b[16] = phase;
        st.b[26] = anim;
        let mut expected = st.b;
        expected[26] = expected_anim;
        expected[11] = expected[11].wrapping_add(sprite_delta);
        sim.handler_0x58b43(&mut st);
        assert_eq!(st.b, expected);
        assert_eq!(sim.unported, vec![0x58B43]);
        assert_eq!(
            (st.px, st.py, sim.move_flags, sim.facing),
            (0x1234, 0x5678, 0x97, 99)
        );
    }
}
#[test]
fn handler_0x58b43_real_anim34_dispatch_and_dead_freeze() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0, 34).unwrap().handler, 0x58B43);
    for dead in [0, 255] {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.fff0 = 3;
        sim.phase = false;
        sim.mario_dead = dead;
        let mut actor = Actor::default();
        actor.position = Position { x: 100, y: 100 };
        actor.activity = 2;
        actor.definition = 34;
        sim.slots[1].b = actor.to_raw_slot();
        let mario = MarioContact {
            x: 100,
            travelling: true,
            ..Default::default()
        };
        sim.step(100, 100, &layout, &tables, &mario, false);
        assert_eq!(
            (sim.slots[1].b[15], sim.slots[1].b[18]),
            if dead == 0 { (2, 77) } else { (0, 0) }
        );
        assert!(sim.unported.is_empty());
    }
}
