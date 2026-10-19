use super::*;
use crate::actor::Position;

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

#[test]
fn handler_0x59487_init_arms_contact_defeat_and_timer() {
    for (frame, next_anim, sprite) in [(3u8, 0u8, 0x20u8), (4, 1, 0x21)] {
        let (mut sim, mut st) = fixture(0);
        sim.fff0 = frame;
        position(&mut st.b, 100, 100);
        st.b[11] = 0x20;
        st.b[12] = 0;
        st.b[26] = 0;
        let mut expected = st.b;
        expected[11] = sprite;
        expected[12] = 128;
        expected[15] = 1;
        expected[18] = 77;
        expected[26] = next_anim;
        expected[28] = 128;
        sim.handler_0x59487(&mut st);
        assert_eq!(st.b, expected);
        assert!(sim.unported.is_empty());
    }
}

#[test]
fn handler_0x59487_rest_then_climb_on_the_bounce_curve() {
    // Timer held: no motion.
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 3;
    position(&mut st.b, 100, 100);
    st.b[28] = 5;
    let mut expected = st.b;
    expected[28] = 4;
    sim.handler_0x59487(&mut st);
    assert_eq!(st.b, expected);

    // Timer expired: one curve step up, accumulated in +24.
    for (phase, dy, next_phase, next_state, timer) in [
        (0u8, 3u8, 1u8, 1u8, 0u8),
        (5, 1, 6, 1, 0),
        (14, 0, 0, 2, 48), // the phase reaches 15 and starts the descent rest
        (15, 0, 0, 2, 48), // cap: phase stays 15, then resets
    ] {
        let (mut sim, mut st) = fixture(1);
        sim.fff0 = 3;
        position(&mut st.b, 100, 100);
        st.b[16] = phase;
        st.b[24] = 10;
        st.b[28] = 0;
        let mut expected = st.b;
        position(&mut expected, 100, 100 - u16::from(dy));
        expected[16] = next_phase;
        expected[24] = 10u8.wrapping_add(dy);
        expected[28] = timer;
        expected[15] = next_state;
        sim.handler_0x59487(&mut st);
        assert_eq!(st.b, expected, "phase {phase}");
        assert!(sim.unported.is_empty());
    }
}

#[test]
fn handler_0x59487_descent_moves_only_on_even_frames_and_wraps() {
    // Timer held.
    let (mut sim, mut st) = fixture(2);
    sim.fff0 = 2;
    position(&mut st.b, 100, 100);
    st.b[28] = 5;
    let mut expected = st.b;
    expected[28] = 4;
    sim.handler_0x59487(&mut st);
    assert_eq!(st.b, expected);

    // Odd frame: no motion even with the timer expired.
    let (mut sim, mut st) = fixture(2);
    sim.fff0 = 3;
    position(&mut st.b, 100, 100);
    st.b[24] = 3;
    st.b[28] = 0;
    let expected = st.b;
    sim.handler_0x59487(&mut st);
    assert_eq!(st.b, expected);

    // Even frame: Y+1 and +24-1; reaching zero starts the 64-frame rest.
    for (leg, next_leg, next_state, timer) in [(3u8, 2u8, 2u8, 0u8), (1, 0, 1, 64), (0, 255, 2, 0)]
    {
        let (mut sim, mut st) = fixture(2);
        sim.fff0 = 2;
        position(&mut st.b, 100, 100);
        st.b[24] = leg;
        st.b[28] = 0;
        let mut expected = st.b;
        position(&mut expected, 100, 101);
        expected[24] = next_leg;
        expected[15] = next_state;
        expected[28] = timer;
        sim.handler_0x59487(&mut st);
        assert_eq!(st.b, expected, "leg {leg}");
        assert!(sim.unported.is_empty());
    }
}

#[test]
fn handler_0x59487_invalid_states_report_the_entry() {
    for state in 3..=255 {
        let (mut sim, mut st) = fixture(state);
        sim.fff0 = 3;
        let original = st.b;
        sim.handler_0x59487(&mut st);
        assert_eq!(st.b, original);
        assert_eq!(sim.unported, vec![0x59487]);
    }
}

#[test]
fn handler_0x59487_real_anim67_step_arms_the_bob() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x00; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0, 67).unwrap().handler, 0x59487);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 3;
    sim.phase = false;
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 100 };
    actor.activity = 2;
    actor.definition = 67;
    actor.state = 0;
    sim.slots[1].b = actor.to_raw_slot();
    let mario = MarioContact {
        x: 1000,
        ..Default::default()
    };
    sim.step(100, 100, &layout, &tables, &mario, false);
    assert_eq!(sim.slots[1].b[15], 1);
    assert_eq!(sim.slots[1].b[18], 77);
    assert_eq!(sim.slots[1].b[12], 128);
    assert_eq!(sim.slots[1].b[28], 128);
    assert!(sim.unported.is_empty());
}
