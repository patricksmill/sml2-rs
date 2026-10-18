use super::*;
use crate::actor::{Actor, Position};

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

fn staging(state: u8, x: u16, y: u16) -> Staging {
    let mut st = Staging::default();
    st.b[5] = 38;
    st.b[15] = state;
    st.b[0..2].copy_from_slice(&x.to_be_bytes());
    st.b[3..5].copy_from_slice(&y.to_be_bytes());
    st
}

fn x_of(slot: &[u8; 32]) -> u16 {
    u16::from_be_bytes([slot[0], slot[1]])
}

fn y_of(slot: &[u8; 32]) -> u16 {
    u16::from_be_bytes([slot[3], slot[4]])
}

#[test]
fn handler_0x5a11e_init_arms_contact_lifts_and_steps_sideways() {
    for (flip, state, x) in [
        (0x00u8, 1u8, 0x00F5u16),
        (0x20, 2, 0x010B),
        (0x40, 1, 0x00F5),
    ] {
        let mut sim = ObjectSim::new(vec![], 0);
        let mut st = staging(0, 0x0100, 0x0080);
        st.b[13] = flip;
        sim.handler_0x5a11e(&mut st);
        assert_eq!(st.b[18], 72, "flip={flip:#04x}");
        assert_eq!(st.b[15], state);
        assert_eq!(x_of(&st.b), x);
        assert_eq!(y_of(&st.b), 0x0080 - 15);
        assert!(sim.unported.is_empty());
    }

    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staging(0, 0x0005, 0x0000);
    sim.handler_0x5a11e(&mut st);
    assert_eq!(x_of(&st.b), 0x0005u16.wrapping_sub(11));
    assert_eq!(y_of(&st.b), 0x0000u16.wrapping_sub(15));
}

#[test]
fn handler_0x5a11e_drifts_every_fourth_frame_and_climbs_the_sprite() {
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staging(1, 0x0100, 0x0080);
    st.b[11] = 0x40;
    sim.fff0 = 1;
    sim.handler_0x5a11e(&mut st);
    assert_eq!((x_of(&st.b), st.b[11], st.b[15]), (0x0100, 0x40, 1));

    sim.fff0 = 2; // 2 % 4 != 0: still no cadence tick
    sim.handler_0x5a11e(&mut st);
    assert_eq!((x_of(&st.b), st.b[11], st.b[15]), (0x0100, 0x40, 1));

    sim.fff0 = 0;
    sim.handler_0x5a11e(&mut st);
    assert_eq!((x_of(&st.b), st.b[11], st.b[15]), (0x00FF, 0x41, 1));

    let mut st = staging(2, 0x0100, 0x0080);
    st.b[11] = 0x40;
    sim.handler_0x5a11e(&mut st);
    assert_eq!((x_of(&st.b), st.b[11], st.b[15]), (0x0101, 0x41, 2));
}

#[test]
fn handler_0x5a11e_sprite_49_switches_to_the_fast_states() {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 0;
    for (state, next) in [(1u8, 3u8), (2, 4)] {
        let mut st = staging(state, 0x0100, 0x0080);
        st.b[11] = 0x48;
        sim.handler_0x5a11e(&mut st);
        assert_eq!((st.b[11], st.b[15]), (0x49, next), "state={state}");
    }

    let mut st = staging(1, 0x0100, 0x0080);
    st.b[11] = 0x49; // past the limit: no further transition
    sim.handler_0x5a11e(&mut st);
    assert_eq!(st.b[15], 1);
}

#[test]
fn handler_0x5a11e_fast_states_move_two_without_a_cadence() {
    for (state, x, want_x) in [
        (3u8, 0x0100u16, 0x00FEu16),
        (4, 0x0100, 0x0102),
        (3, 0x0001, 0xFFFF),
        (4, 0xFFFE, 0x0000),
    ] {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.fff0 = 1; // no cadence gate on states 3/4
        let mut st = staging(state, x, 0x0080);
        sim.handler_0x5a11e(&mut st);
        assert_eq!(x_of(&st.b), want_x, "state={state}, x={x:#06x}");
        assert_eq!(st.b[15], state);
    }
}

#[test]
fn handler_0x5a11e_unknown_state_reports_without_writes() {
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staging(5, 0x0100, 0x0080);
    let before = st.b;
    sim.handler_0x5a11e(&mut st);
    assert_eq!(st.b, before);
    assert_eq!(sim.unported, vec![0x5A11E]);
}

#[test]
fn handler_0x5a11e_real_mode_b_anim38_step() {
    let tables = tables();
    let lay = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x0A, 38).unwrap().handler, 0x5A11E);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    sim.fff0 = 1;
    let mut actor = Actor::default();
    actor.position = Position { x: 0x0100, y: 0x0080 };
    actor.activity = 2;
    actor.definition = 38;
    actor.state = 0;
    sim.slots[1].b = actor.to_raw_slot();
    sim.step(0x0080, 0x0080, &lay, &tables, &MarioContact::default(), false);
    assert_eq!(sim.slots[1].b[18], 72);
    assert_eq!(sim.slots[1].b[15], 1);
    assert_eq!(x_of(&sim.slots[1].b), 0x00F5);
    assert_eq!(y_of(&sim.slots[1].b), 0x0080 - 15);
    assert!(sim.unported.is_empty());
}
