use super::*;
use crate::actor::{Actor, Position};

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

fn layout(metatile: u8) -> LevelLayout {
    LevelLayout::from_bytes(&[metatile; 0x3000]).unwrap()
}

fn staging(state: u8, x: u16, y: u16) -> Staging {
    let mut st = Staging::default();
    st.b[5] = 54;
    st.b[15] = state;
    st.b[0..2].copy_from_slice(&x.to_be_bytes());
    st.b[3..5].copy_from_slice(&y.to_be_bytes());
    st
}

fn y_of(slot: &[u8; 32]) -> u16 {
    u16::from_be_bytes([slot[3], slot[4]])
}

#[test]
fn handler_0x5a8e4_init_faces_away_and_spawns_the_child_before_the_lift() {
    for (mario_x, state, flip) in [(0x0200u16, 2u8, 0x00u8), (0x0050, 1, 0x20)] {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.mario_ref_x = mario_x;
        sim.slot_ptr_hi = 0xAD;
        sim.boss_slot_ptr_lo = 0x20;
        let mut st = staging(0, 0x0100, 0x0080);
        sim.handler_0x5a8e4(&mut st, &layout(0x00));
        assert_eq!(st.b[15], state, "mario_x={mario_x:#06x}");
        assert_eq!(st.b[13], flip);
        assert_eq!(st.b[18], 133);
        assert_eq!(y_of(&st.b), 0x0080 - 16, "parent lifted after the spawn");
        let child = &sim.slots[0].b;
        assert_eq!(child[8], 1);
        assert_eq!(child[5], 55);
        assert_eq!(child[11], 90);
        assert_eq!(child[13], flip);
        assert_eq!(&child[0..5], &[0x01, 0x00, 0x00, 0x00, 0x80][..]);
        assert_eq!(child[6..8], [0xAD, 0x20]);
        assert_eq!(sim.standby, 1);
        assert_eq!((sim.spawn_anim, sim.spawn_sprite, sim.spawn_attr), (0, 0, 0));
        assert_eq!(sim.sfx1, 18);
        assert!(sim.unported.is_empty());
    }
}

#[test]
fn handler_0x5a8e4_walking_states_move_three_on_even_frames() {
    for (state, x, want_x) in [
        (1u8, 0x0100u16, 0x0103u16),
        (2, 0x0100, 0x00FD),
        (1, 0xFFFF, 0x0002),
        (2, 0x0000, 0xFFFD),
    ] {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.fff0 = 0;
        let mut st = staging(state, x, 0x0080);
        let before = st.b;
        sim.handler_0x5a8e4(&mut st, &layout(0x60));
        let mut expected = before;
        expected[0..2].copy_from_slice(&want_x.to_be_bytes());
        expected[3..5].copy_from_slice(&0x0080u16.to_be_bytes());
        assert_eq!(st.b, expected, "state={state}, x={x:#06x}");
    }
}

#[test]
fn handler_0x5a8e4_walking_states_switch_to_fall_only_in_air() {
    for (state, air, next) in [
        (1u8, false, 1u8),
        (1, true, 3),
        (2, false, 2),
        (2, true, 4),
    ] {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.fff0 = 1;
        let mut st = staging(state, 0x0100, 0x0080);
        let before = st.b;
        sim.handler_0x5a8e4(&mut st, &layout(if air { 0x60 } else { 0x00 }));
        assert_eq!(st.b[15], next, "state={state}, air={air}");
        let mut expected = before;
        expected[15] = next;
        assert_eq!(st.b, expected);
    }
}

#[test]
fn handler_0x5a8e4_fall_states_land_back_into_walking() {
    for (state, next) in [(3u8, 1u8), (4, 2)] {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.fff0 = 1;
        let mut st = staging(state, 0x0100, 0x0080);
        st.b[16] = 0;
        sim.handler_0x5a8e4(&mut st, &layout(0x00));
        assert_eq!(st.b[15], next, "grounded state={state}");
        assert_eq!(y_of(&st.b), 0x0080, "phase 0 selects no fall step");

        let mut st = staging(state, 0x0100, 0x0080);
        st.b[16] = 5;
        sim.handler_0x5a8e4(&mut st, &layout(0x60));
        assert_eq!(st.b[15], state, "air state={state}");
        assert_eq!(y_of(&st.b), 0x0080 + 2);
        assert_eq!(st.b[16], 6);
    }
}

#[test]
fn handler_0x5a8e4_fall_states_walk_on_even_frames() {
    for (state, want_x) in [(3u8, 0x0103u16), (4, 0x00FD)] {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.fff0 = 0;
        let mut st = staging(state, 0x0100, 0x0080);
        sim.handler_0x5a8e4(&mut st, &layout(0x60));
        assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), want_x);
        assert_eq!(y_of(&st.b), 0x0080);
        assert_eq!(st.b[15], state);
    }
}

#[test]
fn handler_0x5a8e4_unknown_state_reports_without_writes() {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 1;
    let mut st = staging(5, 0x0100, 0x0080);
    let before = st.b;
    sim.handler_0x5a8e4(&mut st, &layout(0x00));
    assert_eq!(st.b, before);
    assert_eq!(sim.unported, vec![0x5A8E4]);
}

#[test]
fn handler_0x5a8e4_real_mode_b_anim54_step() {
    let tables = tables();
    let lay = layout(0x00);
    assert_eq!(tables.profile_for(0x0A, 54).unwrap().handler, 0x5A8E4);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    sim.slot_ptr_hi = 0xAD;
    sim.boss_slot_ptr_lo = 0x20;
    let mut actor = Actor::default();
    actor.position = Position { x: 0x0100, y: 0x0080 };
    actor.activity = 2;
    actor.definition = 54;
    actor.state = 0;
    sim.slots[1].b = actor.to_raw_slot();
    let mario = MarioContact {
        x: 0x0200,
        ..MarioContact::default()
    };
    sim.step(0x0080, 0x0080, &lay, &tables, &mario, false);
    assert_eq!(sim.slots[1].b[18], 133);
    assert_eq!(sim.slots[1].b[15], 2);
    assert_eq!(y_of(&sim.slots[1].b), 0x0080 - 16);
    assert_eq!(sim.slots[0].b[5], 55);
    assert_eq!(sim.slots[0].b[8], 1);
    assert!(sim.unported.is_empty());
}
