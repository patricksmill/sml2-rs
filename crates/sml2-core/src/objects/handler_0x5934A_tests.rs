use super::*;
use crate::actor::{Actor, Position};

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}
fn fixture(state: u8) -> (ObjectSim, Staging) {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.facing = 99;
    sim.move_flags = 0x97;
    let mut st = Staging::default();
    st.b = std::array::from_fn(|i| (i as u8).wrapping_mul(37) ^ 0xA5);
    st.b[15] = state;
    st.px = 0x1234;
    st.py = 0x5678;
    (sim, st)
}
fn position(slot: &mut [u8; 32], x: u16, y: u16) {
    slot[0..2].copy_from_slice(&x.to_be_bytes());
    slot[3..5].copy_from_slice(&y.to_be_bytes());
}
fn check(sim: &mut ObjectSim, st: &mut Staging, t: &ObjectTables, expected: [u8; 32], facing: u8) {
    sim.handler_0x5934a(st, t);
    assert_eq!(st.b, expected);
    assert_eq!(
        (st.px, st.py, sim.move_flags, sim.facing),
        (0x1234, 0x5678, 0x97, facing)
    );
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5934a_state_three_uses_anim62_sprite_and_timer() {
    let t = tables();
    for frame in [0u8, 1, 3, 7, 8, 255] {
        for timer in [0u8, 1, 2, 48, 255] {
            let (mut sim, mut st) = fixture(3);
            position(&mut st.b, 0x1234, 0x5678);
            st.b[28] = timer;
            sim.fff0 = frame;
            sim.mario_ref_x = 0x1234;
            let mut expected = st.b;
            if frame % 8 == 0 {
                expected[11] ^= 1; // $5B511 toggles bit 0 every eighth frame.
            }
            expected[28] = timer.wrapping_sub(1);
            if timer == 1 {
                expected[15] = 4;
                expected[18] = 82;
                expected[11] = 0x70; // $5936D: anim 62's sprite, not $6E.
                expected[28] = 0x30; // $59373: 48 frames, not 24.
            }
            check(&mut sim, &mut st, &t, expected, 99);
        }
    }
}

#[test]
fn handler_0x5934a_state_four_wind_down_faces_and_sets_leg_eight() {
    let t = tables();
    for frame in [0u8, 1, 3, 4, 7, 8, 255] {
        for timer in [0u8, 1, 2, 255] {
            for (target_x, next_state, flip, facing) in
                [(0x1233, 1, 0, 255), (0x1234, 1, 0, 0), (0x1235, 2, 32, 1)]
            {
                let (mut sim, mut st) = fixture(4);
                position(&mut st.b, 0x1234, 0x5678);
                st.b[28] = timer;
                sim.fff0 = frame;
                sim.mario_ref_x = target_x;
                let mut expected = st.b;
                if frame % 4 == 0 {
                    expected[11] ^= 1; // $5B51A toggles bit 0 every fourth frame.
                }
                expected[28] = timer.wrapping_sub(1);
                let mut expected_facing = 99;
                if timer == 1 {
                    expected[15] = next_state;
                    expected[13] = flip;
                    expected[24] = 8;
                    expected_facing = facing;
                }
                check(&mut sim, &mut st, &t, expected, expected_facing);
            }
        }
    }
}

#[test]
fn handler_0x5934a_init_reuses_the_shared_emergence_body() {
    let t = tables();
    // Independent signed ROM bytes $59082..$59091.
    let ys: [i16; 8] = [4, -20, -32, 40, 28, 16, -44, -8];
    let xs: [i16; 8] = [0, -16, 24, -24, 32, -40, -16, 40];
    for direction in 0..8u8 {
        let (mut sim, mut st) = fixture(0);
        sim.div = 0xA0 | direction;
        st.b[31] = 0;
        position(&mut st.b, 0x12FF, 0x3400);
        let mut expected = st.b;
        position(
            &mut expected,
            0x12FFu16.wrapping_add_signed(xs[direction as usize]),
            0x3400u16.wrapping_add_signed(ys[direction as usize]),
        );
        expected[13] = 0;
        expected[14] = 16;
        expected[15] = 3;
        expected[18] = 0;
        expected[28] = 48;
        expected[31] |= 1 << direction;
        check(&mut sim, &mut st, &t, expected, 99);
    }

    // Full direction mask: $59136 requests removal mode 2 without rewards.
    let (mut sim, mut st) = fixture(0);
    sim.spawned = vec![true];
    sim.standby = 0;
    sim.active = 0;
    st.b[8] = 2;
    st.b[31] = 255;
    sim.handler_0x5934a(&mut st, &t);
    let mut expected = [0; 32];
    let hb = t.hitbox_for(0, 0).unwrap();
    expected[19..23].copy_from_slice(&hb.map(|v| v as u8));
    assert_eq!(st.b, expected);
    assert_eq!((sim.standby, sim.active), (0, 255));
    assert_eq!((sim.done, sim.remove_mode), (2, 0));
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5934a_tracking_states_move_and_face() {
    let t = tables();
    for (state, x, y, target_x, target_y, leg, nx, ny, next_leg, next_state, flip, facing) in [
        (1, 0u16, 0u16, 65533, 0, 255, 0, 65533, 0, 2, 32, 1),
        (2, 65535, 65535, 65535, 65535, 16, 65535, 2, 16, 1, 0, 0),
        (1, 256, 200, 253, 201, 7, 253, 200, 8, 1, 0, 0),
        (2, 255, 200, 257, 199, 7, 257, 199, 6, 1, 0, 0),
    ] {
        let (mut sim, mut st) = fixture(state);
        position(&mut st.b, x, y);
        st.b[24] = leg;
        sim.mario_ref_x = target_x;
        sim.mario_ref_y = target_y;
        let mut expected = st.b;
        position(&mut expected, nx, ny);
        expected[11] ^= 1;
        expected[24] = next_leg;
        expected[15] = next_state;
        expected[13] = flip;
        check(&mut sim, &mut st, &t, expected, facing);
    }
}

#[test]
fn handler_0x5934a_invalid_states_report_the_entry() {
    let t = tables();
    for state in 5..=255 {
        let (mut sim, mut st) = fixture(state);
        let original = st.b;
        sim.handler_0x5934a(&mut st, &t);
        assert_eq!(st.b, original);
        assert_eq!(sim.unported, vec![0x5934A]);
    }
}

#[test]
fn handler_0x5934a_real_anim62_step_and_sibling_anim53_stay_distinct() {
    let t = tables();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(t.profile_for(0, 62).unwrap().handler, 0x5934A);
    assert_eq!(t.profile_for(0, 53).unwrap().handler, 0x5902D);
    for (anim, sprite, timer) in [(62u8, 0x70u8, 0x30u8), (53, 110, 24)] {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.fff0 = 3; // step's phase toggle makes it 4: no state-3 sprite toggle.
        sim.phase = false; // after the toggle update_phase = 1; slot 1 matches.
        let mut actor = Actor::default();
        actor.position = Position { x: 100, y: 100 };
        actor.activity = 2;
        actor.definition = anim;
        actor.state = 3;
        actor.timer = 1;
        actor.sprite = 0x55;
        sim.slots[1].b = actor.to_raw_slot();
        let mario = MarioContact {
            x: 1000,
            ..Default::default()
        };
        sim.step(100, 100, &layout, &t, &mario, false);
        let result = &sim.slots[1].b;
        assert_eq!(result[15], 4, "anim {anim}");
        assert_eq!(result[18], 82, "anim {anim}");
        assert_eq!(result[11], sprite, "anim {anim}");
        assert_eq!(result[28], timer, "anim {anim}");
        assert!(sim.unported.is_empty());
    }
}
