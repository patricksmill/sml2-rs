use super::*;
use crate::actor::{Actor, InteractionFlags, Position};

fn fixture(state: u8) -> (ObjectSim, Staging) {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.facing = 99;
    let mut st = Staging::default();
    st.b = std::array::from_fn(|i| (i as u8).wrapping_mul(37) ^ 0xA5);
    st.b[15] = state;
    st.b[26] = 0; // valid $5B5B6 animation frame
    st.px = 0x1234;
    st.py = 0x5678;
    (sim, st)
}
fn position(slot: &mut [u8; 32], x: u16, y: u16) {
    slot[0..2].copy_from_slice(&x.to_be_bytes());
    slot[3..5].copy_from_slice(&y.to_be_bytes());
}
fn sprite_cycle(slot: &mut [u8; 32], frame: u8) {
    if frame % 2 != 0 {
        return;
    }
    let next = (slot[26] + 1) % 3;
    slot[26] = next;
    slot[11] = slot[11].wrapping_add([0xFE, 1, 1][usize::from(next)]);
}
fn check(sim: &mut ObjectSim, st: &mut Staging, expected: [u8; 32]) {
    sim.handler_0x591a3(st);
    assert_eq!(st.b, expected);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x591a3_waits_for_a_rider_with_contact_and_flash() {
    for (frame, toggled) in [(0u8, true), (1, false), (7, false), (8, true)] {
        let (mut sim, mut st) = fixture(0);
        sim.fff0 = frame;
        position(&mut st.b, 100, 100);
        st.b[11] = 0x20;
        st.b[14] = 0;
        st.b[18] = 0;
        st.b[23] = 0;
        let mut expected = st.b;
        expected[14] = 16;
        expected[18] = 32;
        if toggled {
            expected[11] ^= 1;
        }
        check(&mut sim, &mut st, expected);
        assert_eq!(sim.sfx1, 0);
    }
}

#[test]
fn handler_0x591a3_starts_the_lift_when_ridden() {
    let (mut sim, mut st) = fixture(0);
    sim.fff0 = 3;
    position(&mut st.b, 100, 100);
    st.b[14] = 0;
    st.b[18] = 0;
    st.b[23] = 0x08;
    let mut expected = st.b;
    expected[11] = 98;
    expected[14] = 16;
    expected[15] = 1;
    expected[18] = 32;
    expected[24] = 96;
    expected[25] = 96;
    check(&mut sim, &mut st, expected);
    assert_eq!(sim.sfx1, 22);
}

#[test]
fn handler_0x591a3_rise_counts_down_and_stops_at_zero() {
    for (leg, next_leg, moved) in [(5u8, 4u8, true), (1, 0, true), (0, 0, false)] {
        let (mut sim, mut st) = fixture(1);
        sim.fff0 = 3;
        position(&mut st.b, 100, 100);
        st.b[23] = 0x08;
        st.b[24] = leg;
        st.b[25] = 96;
        sim.mario.y = 100;
        let mut expected = st.b;
        if moved {
            position(&mut expected, 100, 99);
        }
        expected[24] = next_leg;
        check(&mut sim, &mut st, expected);
        assert_eq!(sim.mario_y_adjust, moved.then_some(99));
    }
}

#[test]
fn handler_0x591a3_step_off_waits_then_descends_to_the_target() {
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 3;
    position(&mut st.b, 100, 100);
    st.b[23] = 0;
    st.b[24] = 5;
    st.b[25] = 96;
    let mut expected = st.b;
    expected[15] = 2;
    expected[28] = 16;
    check(&mut sim, &mut st, expected);

    // Timer held.
    let (mut sim, mut st) = fixture(2);
    sim.fff0 = 3;
    position(&mut st.b, 100, 100);
    st.b[23] = 0;
    st.b[24] = 5;
    st.b[25] = 96;
    st.b[28] = 5;
    let mut expected = st.b;
    expected[28] = 4;
    check(&mut sim, &mut st, expected);

    // Timer reaches zero and the same call descends.
    let (mut sim, mut st) = fixture(2);
    sim.fff0 = 3;
    position(&mut st.b, 100, 100);
    st.b[23] = 0;
    st.b[24] = 5;
    st.b[25] = 96;
    st.b[28] = 1;
    sim.mario.y = 100;
    let mut expected = st.b;
    position(&mut expected, 100, 101);
    expected[24] = 6;
    expected[28] = 0;
    check(&mut sim, &mut st, expected);
    assert_eq!(sim.mario_y_adjust, None);

    // At the target: no further descent.
    let (mut sim, mut st) = fixture(2);
    sim.fff0 = 3;
    position(&mut st.b, 100, 100);
    st.b[23] = 0;
    st.b[24] = 96;
    st.b[25] = 96;
    st.b[28] = 0;
    let expected = st.b;
    check(&mut sim, &mut st, expected);
    assert_eq!(sim.mario_y_adjust, None);
}

#[test]
fn handler_0x591a3_resumes_when_the_rider_returns() {
    let (mut sim, mut st) = fixture(2);
    sim.fff0 = 3;
    position(&mut st.b, 100, 100);
    st.b[23] = 0x08;
    st.b[24] = 5;
    st.b[25] = 96;
    st.b[28] = 9;
    let mut expected = st.b;
    expected[15] = 1;
    expected[28] = 0;
    check(&mut sim, &mut st, expected);
    assert_eq!(sim.mario_y_adjust, None);
}

#[test]
fn handler_0x591a3_unridden_descent_leaves_the_player_alone() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    // $58E57: an expired wait moves only the lift ($3D3D), including on the
    // call that decrements the wait to zero. The final step reaches +25.
    for (wait, descends) in [(0, true), (1, true), (2, false)] {
        for lift_y in [0x00FFu16, 0xFFFF] {
            let (mut sim, mut st) = fixture(2);
            sim.fff0 = 3; // $5B5B6 does not animate on an odd frame.
            position(&mut st.b, 100, lift_y);
            st.b[23] = 0;
            st.b[24] = 95;
            st.b[25] = 96;
            st.b[28] = wait;
            let mut player = crate::player_sim::PlayerSim::new();
            player.x = 200;
            player.y = 568;
            player.vvel = 8;
            player.on_ground = false;
            sim.mario.x = player.x;
            sim.mario.y = player.y;
            let mut expected = st.b;
            expected[28] = wait.saturating_sub(1);
            if descends {
                expected[3..5].copy_from_slice(&match lift_y {
                    0x00FF => [0x01, 0x00],
                    _ => [0x00, 0x00],
                });
                expected[24] = 96;
            }

            sim.dispatch_handler(0x591A3, &mut st, &layout, &tables);
            assert_eq!(st.b, expected);
            assert_eq!(sim.mario_y_adjust, None);
            player.apply_object_result(&mut sim, false);
            assert_eq!(player.y.to_le_bytes(), [0x38, 0x02]);
            assert_eq!((player.vvel, player.on_ground_obj), (8, false));
            assert!(sim.unported.is_empty());
        }
    }
}

#[test]
fn handler_0x591a3_sprite_cycle_runs_on_both_live_states() {
    for state in [1u8, 2] {
        for (frame, changes) in [(2u8, true), (3, false), (4, true), (6, true)] {
            let (mut sim, mut st) = fixture(state);
            sim.fff0 = frame;
            position(&mut st.b, 100, 100);
            st.b[11] = 0x20;
            st.b[23] = 0;
            st.b[24] = 96;
            st.b[25] = 96;
            st.b[26] = 0;
            st.b[28] = 0;
            let mut expected = st.b;
            if state == 1 {
                expected[15] = 2;
                expected[28] = 16;
            }
            if changes {
                sprite_cycle(&mut expected, frame);
            }
            check(&mut sim, &mut st, expected);
        }
    }
}

#[test]
fn handler_0x591a3_invalid_states_report_the_entry() {
    for state in 3..=255 {
        let (mut sim, mut st) = fixture(state);
        sim.fff0 = 3;
        let original = st.b;
        sim.handler_0x591a3(&mut st);
        assert_eq!(st.b, original);
        assert_eq!(sim.unported, vec![0x591A3]);
    }
}

#[test]
fn handler_0x591a3_real_anim55_step_starts_the_lift() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0, 55).unwrap().handler, 0x591A3);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 3;
    sim.phase = false;
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 100 };
    actor.activity = 2;
    actor.definition = 55;
    actor.state = 0;
    actor.interactions = InteractionFlags::from_bits(0x08);
    sim.slots[1].b = actor.to_raw_slot();
    let mario = MarioContact {
        x: 1000,
        y: 100,
        ..Default::default()
    };
    sim.step(100, 100, &layout, &tables, &mario, false);
    assert_eq!(sim.slots[1].b[15], 1);
    assert_eq!(sim.slots[1].b[18], 32);
    assert_eq!(sim.slots[1].b[24], 96);
    assert_eq!(sim.slots[1].b[25], 96);
    assert_eq!(sim.sfx1, 22);
    assert!(sim.unported.is_empty());
}
