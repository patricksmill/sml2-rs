use super::*;
use crate::actor::Position;

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

fn staging(state: u8, x: u16, y: u16) -> Staging {
    let mut st = Staging::default();
    st.b[5] = 82;
    st.b[15] = state;
    st.b[0..2].copy_from_slice(&x.to_be_bytes());
    st.b[3..5].copy_from_slice(&y.to_be_bytes());
    st
}

#[test]
fn handler_0x59728_init_faces_from_the_flip_bit_and_requests_sfx2() {
    for (flip, state) in [(0x00u8, 1u8), (0x20, 2)] {
        let mut sim = ObjectSim::new(vec![], 0);
        let mut st = staging(0, 0x0100, 0x0080);
        st.b[13] = flip;
        sim.handler_0x59728(&mut st);
        assert_eq!(st.b[18], 77);
        assert_eq!(st.b[15], state);
        assert_eq!(sim.sfx2, 11);
        assert!(sim.unported.is_empty());
    }
    // Any state other than 1/2 re-arms the flyer.
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staging(3, 0x0100, 0x0080);
    st.b[13] = 0x20;
    sim.handler_0x59728(&mut st);
    assert_eq!((st.b[18], st.b[15]), (77, 2));
}

#[test]
fn handler_0x59728_states_move_diagonally_every_other_frame() {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 3; // odd: no cycle, no move
    let mut st = staging(1, 0x0100, 0x0080);
    st.b[11] = 0x10;
    sim.handler_0x59728(&mut st);
    assert_eq!(
        (
            u16::from_be_bytes([st.b[0], st.b[1]]),
            u16::from_be_bytes([st.b[3], st.b[4]]),
            st.b[11]
        ),
        (0x0100, 0x0080, 0x10)
    );

    sim.fff0 = 2;
    sim.handler_0x59728(&mut st);
    assert_eq!(
        (
            u16::from_be_bytes([st.b[0], st.b[1]]),
            u16::from_be_bytes([st.b[3], st.b[4]]),
            st.b[11]
        ),
        (0x00FE, 0x007F, 0x11)
    );

    let mut st = staging(2, 0x0100, 0x0080);
    st.b[11] = 0x10;
    sim.handler_0x59728(&mut st);
    assert_eq!(
        (
            u16::from_be_bytes([st.b[0], st.b[1]]),
            u16::from_be_bytes([st.b[3], st.b[4]]),
            st.b[11]
        ),
        (0x0102, 0x007F, 0x11)
    );
}

#[test]
fn handler_0x59728_real_mode_a_anim82_step() {
    let tables = tables();
    let lay = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x00, 82).unwrap().handler, 0x59728);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x00);
    sim.mario_ref_x = 0x0050;
    let mut actor = crate::actor::Actor::default();
    actor.position = Position {
        x: 0x0100,
        y: 0x0080,
    };
    actor.activity = 2;
    actor.definition = 82;
    actor.state = 0;
    sim.slots[1].b = actor.to_raw_slot();
    sim.step(
        0x0080,
        0x0080,
        &lay,
        &tables,
        &MarioContact::default(),
        false,
    );
    assert_eq!((sim.slots[1].b[18], sim.slots[1].b[15]), (77, 1));
    assert!(sim.unported.is_empty());
}
